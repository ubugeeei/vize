<script setup lang="ts">
import { ref, watch, computed, onMounted, onUnmounted } from "vue";
import MonacoEditor from "./MonacoEditor.vue";
import * as monaco from "monaco-editor";
import type {
  WasmModule,
  TypeCheckResult,
  TypeCheckDiagnostic,
  TypeCheckCapabilities,
} from "../wasm/index";

interface Diagnostic {
  message: string;
  help?: string;
  code?: number;
  startLine: number;
  startColumn: number;
  endLine?: number;
  endColumn?: number;
  severity: "error" | "warning" | "info";
}

// Generate help suggestions based on TypeScript error code and message
function generateHelp(code: number, message: string): string | undefined {
  // Common TypeScript errors with helpful suggestions
  switch (code) {
    // ========== Name Resolution Errors ==========
    case 2304: {
      // Cannot find name 'X'
      const nameMatch = message.match(/Cannot find name '(\w+)'/);
      if (nameMatch) {
        const name = nameMatch[1];
        if (name.startsWith("$")) {
          return `**\`${name}\`** is a Vue template variable.\n\n**Why:** Template variables like \`$event\`, \`$refs\`, \`$slots\` are only available in \`<template>\`.\n\n**Fix:** Use it inside template, or access via script APIs:\n\`\`\`ts\n// In <script setup>\nconst slots = useSlots()\nconst attrs = useAttrs()\n\`\`\``;
        }
        if (
          [
            "ref",
            "reactive",
            "computed",
            "watch",
            "watchEffect",
            "onMounted",
            "onUnmounted",
            "toRef",
            "toRefs",
          ].includes(name)
        ) {
          return `**\`${name}\`** is a Vue Composition API function.\n\n**Fix:** Import from \`vue\`:\n\`\`\`ts\nimport { ${name} } from 'vue'\n\n// Usage example\nconst count = ref(0)\nconst doubled = computed(() => count.value * 2)\n\`\`\``;
        }
        return `**\`${name}\`** is not defined.\n\n**Possible causes:**\n- Not imported from a module\n- Not declared in this scope\n- Typo in the name\n\n**Fix options:**\n\`\`\`ts\n// 1. Import from module\nimport { ${name} } from './module'\n\n// 2. Declare locally\nconst ${name} = someValue\n\n// 3. Import from package\nimport { ${name} } from 'package-name'\n\`\`\``;
      }
      break;
    }
    case 2552: {
      // Cannot find name 'X'. Did you mean 'Y'?
      const meanMatch = message.match(/Did you mean '(\w+)'/);
      if (meanMatch) {
        return `**Typo detected.**\n\n**Suggestion:** Did you mean **\`${meanMatch[1]}\`**?\n\n**Fix:**\n\`\`\`ts\n// Change to:\n${meanMatch[1]}\n\`\`\``;
      }
      break;
    }

    // ========== Type Mismatch Errors ==========
    case 2322: {
      // Type 'X' is not assignable to type 'Y'
      const typeMatch = message.match(
        /Type '(.+?)' is not assignable to type '(.+?)'/,
      );
      if (typeMatch) {
        const [, fromType, toType] = typeMatch;
        if (fromType === "string" && toType === "number") {
          return `**Type mismatch:** \`string\` cannot be assigned to \`number\`.\n\n**Why:** TypeScript requires explicit type conversion.\n\n**Fix options:**\n\`\`\`ts\n// parseInt for integers\nconst num = parseInt(str, 10)\n\n// parseFloat for decimals\nconst num = parseFloat(str)\n\n// Number constructor\nconst num = Number(str)\n\n// With fallback for NaN\nconst num = Number(str) || 0\n\n// Unary plus (shortest)\nconst num = +str\n\`\`\``;
        }
        if (fromType === "number" && toType === "string") {
          return `**Type mismatch:** \`number\` cannot be assigned to \`string\`.\n\n**Fix options:**\n\`\`\`ts\n// String constructor\nconst str = String(num)\n\n// toString method\nconst str = num.toString()\n\n// Template literal (recommended)\nconst str = \`\${num}\`\n\n// With formatting\nconst str = num.toFixed(2) // "123.45"\n\`\`\``;
        }
        return `**Type mismatch:** \`${fromType}\` cannot be assigned to \`${toType}\`.\n\n**Fix options:**\n\`\`\`ts\n// 1. Fix the value to match expected type\nconst value: ${toType} = correctValue\n\n// 2. Type assertion (if you're sure)\nconst value = someValue as ${toType}\n\n// 3. Update type definition to accept both\ntype MyType = ${fromType} | ${toType}\n\`\`\``;
      }
      return `**Type mismatch.** The value type doesn't match the expected type.\n\n**Fix:** Check the type definition and ensure the value matches.`;
    }
    case 2345: {
      // Argument type mismatch
      const argMatch = message.match(
        /Argument of type '(.+?)' is not assignable to parameter of type '(.+?)'/,
      );
      if (argMatch) {
        return `**Argument type mismatch.**\n\n**Expected:** \`${argMatch[2]}\`\n**Received:** \`${argMatch[1]}\`\n\n**Fix options:**\n\`\`\`ts\n// 1. Convert the argument\nfunc(convertedValue)\n\n// 2. Type assertion (if compatible)\nfunc(value as ${argMatch[2]})\n\n// 3. Update function to accept the type\nfunction func(param: ${argMatch[1]} | ${argMatch[2]}) { }\n\`\`\``;
      }
      return `**Argument type mismatch.** The argument doesn't match the function parameter type.\n\n**Fix:** Check the function signature and convert the argument if needed.`;
    }
    case 2349: // This expression is not callable
      return `**Expression is not callable.**\n\n**Why:** You're trying to call something that isn't a function.\n\n**Common causes:**\n- Value is \`undefined\` or \`null\`\n- It's an object or primitive, not a function\n- Property returns a value, not a method\n\n**Fix options:**\n\`\`\`ts\n// 1. Check if it's a function first\nif (typeof maybeFunc === 'function') {\n  maybeFunc()\n}\n\n// 2. Use optional chaining for optional calls\nmaybeFunc?.()\n\n// 3. Provide default function\nconst fn = maybeFunc ?? (() => {})\nfn()\n\`\`\``;

    // ========== Property Access Errors ==========
    case 2339: {
      // Property 'X' does not exist on type 'Y'
      const propMatch = message.match(
        /Property '(\w+)' does not exist on type '(.+?)'/,
      );
      if (propMatch) {
        const [, prop, type] = propMatch;
        if (type.includes("Ref<")) {
          return `**Ref access error.**\n\n**Why:** \`Ref\` wraps the value in \`.value\` property.\n\n**Fix:** Access through \`.value\`:\n\`\`\`ts\n// Wrong\nmyRef.${prop}\n\n// Correct\nmyRef.value.${prop}\n\n// In template (auto-unwrapped)\n{{ myRef.${prop} }}\n\`\`\``;
        }
        return `**Property \`${prop}\` doesn't exist** on type \`${type}\`.\n\n**Possible causes:**\n- Typo in property name\n- Property not defined in type\n- Accessing wrong object\n\n**Fix options:**\n\`\`\`ts\n// 1. Check if property exists\nif ('${prop}' in obj) {\n  obj.${prop}\n}\n\n// 2. Optional chaining (returns undefined if missing)\nobj?.${prop}\n\n// 3. Extend the type definition\ninterface Extended extends Original {\n  ${prop}: SomeType\n}\n\n// 4. Index signature access\nobj['${prop}']\n\`\`\``;
      }
      break;
    }
    case 2551: {
      // Property 'X' does not exist. Did you mean 'Y'?
      const suggestMatch = message.match(/Did you mean '(\w+)'/);
      if (suggestMatch) {
        return `**Typo detected in property name.**\n\n**Suggestion:** Did you mean **\`${suggestMatch[1]}\`**?\n\n**Fix:**\n\`\`\`ts\n// Change to:\nobj.${suggestMatch[1]}\n\`\`\``;
      }
      break;
    }

    // ========== Null/Undefined Errors ==========
    case 2532: // Object is possibly 'undefined'
      return `**Value may be \`undefined\`.**\n\n**Why:** TypeScript detected this value could be \`undefined\` at runtime.\n\n**Fix options:**\n\`\`\`ts\n// 1. Optional chaining (safe access)\nobj?.property\nobj?.method()\n\n// 2. Nullish coalescing (provide default)\nconst value = obj ?? defaultValue\n\n// 3. Explicit undefined check\nif (obj !== undefined) {\n  obj.property // OK, obj is defined here\n}\n\n// 4. Non-null assertion (only if you're 100% sure)\nobj!.property // Tells TS "trust me, it's defined"\n\`\`\``;
    case 2531: // Object is possibly 'null'
      return `**Value may be \`null\`.**\n\n**Why:** TypeScript detected this value could be \`null\` at runtime.\n\n**Fix options:**\n\`\`\`ts\n// 1. Optional chaining\nobj?.property\n\n// 2. Nullish coalescing\nconst value = obj ?? defaultValue\n\n// 3. Explicit null check\nif (obj !== null) {\n  obj.property // OK\n}\n\n// 4. Combined check\nif (obj != null) { // checks both null and undefined\n  obj.property\n}\n\`\`\``;
    case 2533: // Object is possibly 'null' or 'undefined'
      return `**Value may be \`null\` or \`undefined\`.**\n\n**Fix:**\n\`\`\`ts\n// Optional chaining (recommended)\nobj?.property\nobj?.method?.()\n\n// With default value\nconst value = obj?.property ?? 'default'\n\n// Explicit check\nif (obj) {\n  obj.property // OK\n}\n\`\`\``;
    case 18048: // 'X' is possibly 'undefined'
      return `**Value may be \`undefined\`.**\n\n**Fix options:**\n\`\`\`ts\n// 1. Provide default value\nconst value = maybeUndefined ?? 'default'\n\n// 2. Initialize with value\nconst data = ref<string>('initial') // Not undefined\n\n// 3. Check before use\nif (value !== undefined) {\n  // use value safely\n}\n\n// 4. Array methods with fallback\nconst first = arr[0] ?? defaultItem\n\`\`\``;

    // ========== Type Unknown/Any Errors ==========
    case 2571: // Object is of type 'unknown'
      return `**Type is \`unknown\`.**\n\n**Why:** \`unknown\` is the type-safe version of \`any\`. You must narrow the type before using it.\n\n**Fix options:**\n\`\`\`ts\n// 1. typeof type guard\nif (typeof value === 'string') {\n  value.toUpperCase() // OK, value is string\n}\nif (typeof value === 'number') {\n  value.toFixed(2) // OK, value is number\n}\n\n// 2. instanceof check\nif (value instanceof Error) {\n  value.message // OK\n}\n\n// 3. Custom type guard\nfunction isUser(v: unknown): v is User {\n  return (\n    typeof v === 'object' &&\n    v !== null &&\n    'name' in v &&\n    'email' in v\n  )\n}\nif (isUser(value)) {\n  value.name // OK\n}\n\n// 4. Type assertion (less safe)\nconst user = value as User\n\`\`\``;
    case 7006: {
      // Parameter implicitly has an 'any' type
      const paramMatch = message.match(/Parameter '(\w+)'/);
      const paramName = paramMatch ? paramMatch[1] : "param";
      return `**Parameter \`${paramName}\` needs a type.**\n\n**Why:** TypeScript cannot infer the type and defaults to \`any\`.\n\n**Fix options:**\n\`\`\`ts\n// 1. Add explicit type annotation\nfunction example(${paramName}: string) {\n  return ${paramName}.toUpperCase()\n}\n\n// 2. Arrow function with type\nconst fn = (${paramName}: number) => ${paramName} * 2\n\n// 3. Default value (type inferred)\nfunction example(${paramName} = 'default') {\n  // ${paramName} is inferred as string\n}\n\n// 4. Object parameter with type\nfunction example({ ${paramName} }: { ${paramName}: string }) { }\n\`\`\``;
    }
    case 7031: // Binding element implicitly has an 'any' type
      return `**Destructured value needs a type.**\n\n**Why:** TypeScript cannot infer types in destructuring patterns.\n\n**Fix options:**\n\`\`\`ts\n// 1. Type the entire pattern\nconst { name, age }: { name: string; age: number } = obj\n\n// 2. Use an interface\ninterface Person {\n  name: string\n  age: number\n}\nconst { name, age }: Person = obj\n\n// 3. Function parameter destructuring\nfunction greet({ name, age }: Person) {\n  console.log(\`\${name} is \${age}\`)\n}\n\n// 4. With Vue defineProps\nconst { title, count } = defineProps<{\n  title: string\n  count: number\n}>()\n\`\`\``;
    case 7005: // Variable implicitly has an 'any' type
      return `**Variable type is implicitly \`any\`.**\n\n**Why:** TypeScript couldn't infer the type.\n\n**Fix options:**\n\`\`\`ts\n// 1. Add explicit type\nlet value: string\nlet items: number[]\nlet user: User | null = null\n\n// 2. Initialize with value (type inferred)\nlet value = 'hello' // string\nlet count = 0 // number\n\n// 3. Empty array with type\nconst items: string[] = []\nconst map: Map<string, number> = new Map()\n\n// 4. Generic type parameters\nconst ref = ref<User | null>(null)\n\`\`\``;

    // ========== Function Errors ==========
    case 2554: {
      // Expected X arguments, but got Y
      const argMatch = message.match(
        /Expected (\d+) arguments?, but got (\d+)/,
      );
      if (argMatch) {
        const [, expected, got] = argMatch;
        const expectedNum = parseInt(expected);
        const gotNum = parseInt(got);

        // Vue event handler pattern: Expected 0 arguments, but got 1
        // This happens when @click="handler" passes $event but handler takes no args
        if (expectedNum === 0 && gotNum === 1) {
          return `**Event handler argument mismatch.**\n\n**Why:** In Vue templates, \`@click="handler"\` automatically passes the event object (\`$event\`) as the first argument. But your function expects 0 arguments.\n\n**Fix options:**\n\`\`\`ts\n// Option 1: Call the function explicitly (don't pass event)\n// @click="handler()"\n<button @click="handler()">Click</button>\n\n// Option 2: Use arrow function wrapper\n// @click="() => handler()"\n<button @click="() => handler()">Click</button>\n\n// Option 3: Accept the event parameter\nfunction handler(event?: Event) {\n  // event is optional, use if needed\n}\n// Then @click="handler" works\n\`\`\`\n\n**Note:** \`@click="handler"\` is equivalent to \`@click="handler($event)"\``;
        }

        // General case: wrong number of arguments
        return `**Wrong number of arguments.**\n\n**Expected:** ${expected} argument(s)\n**Provided:** ${got} argument(s)\n\n**Fix:**\n\`\`\`ts\n// Check the function signature\nfunction example(a: string, b: number, c?: boolean) {\n  // a, b are required\n  // c is optional\n}\n\n// Call with correct arguments\nexample('hello', 42) // OK\nexample('hello', 42, true) // OK\n\`\`\``;
      }
      break;
    }
    case 2555: {
      // Expected at least X arguments, but got Y
      const argMatch = message.match(
        /Expected at least (\d+) arguments?, but got (\d+)/,
      );
      if (argMatch) {
        return `**Not enough arguments.**\n\n**Required:** at least ${argMatch[1]} argument(s)\n**Provided:** ${argMatch[2]} argument(s)\n\n**Fix:** Provide all required arguments:\n\`\`\`ts\n// Function with required and optional params\nfunction example(required1: string, required2: number, optional?: boolean) { }\n\n// Must provide at least required params\nexample('hello', 42) // OK\n\`\`\``;
      }
      return `**Not enough arguments.** Check the function signature for required parameters.`;
    }
    case 2556: // A spread argument must either have a tuple type
      return `**Spread argument type error.**\n\n**Why:** TypeScript needs to know the exact types when spreading.\n\n**Fix options:**\n\`\`\`ts\n// 1. Use tuple type\nconst args: [string, number] = ['hello', 42]\nfunc(...args) // OK\n\n// 2. Use 'as const' for literal types\nconst args = ['hello', 42] as const\nfunc(...args) // OK\n\n// 3. Type assertion\nconst args = ['hello', 42] as [string, number]\nfunc(...args)\n\n// 4. Rest parameters in function\nfunction func(...args: [string, number]) { }\n\`\`\``;

    // ========== Module/Import Errors ==========
    case 2307: {
      // Cannot find module 'X'
      const modMatch = message.match(/Cannot find module '([^']+)'/);
      const modName = modMatch ? modMatch[1] : "module";
      const pkgName = modName.startsWith(".") ? null : modName.split("/")[0];
      return `**Module not found:** \`${modName}\`\n\n**Fix options:**\n\`\`\`ts\n// 1. Install the package${pkgName ? `\n// npm install ${pkgName}` : ""}\n\n// 2. Install type definitions${pkgName ? `\n// npm install -D @types/${pkgName}` : ""}\n\n// 3. For local modules, check path\nimport { something } from './correct/path'\n\n// 4. Add module declaration\ndeclare module '${modName}' {\n  export const value: string\n}\n\`\`\``;
    }
    case 2306: // 'X' is not a module
      return `**File is not a module.**\n\n**Why:** This file doesn't have any exports.\n\n**Fix:** Add exports to the file:\n\`\`\`ts\n// Named exports\nexport const myValue = 'hello'\nexport function myFunc() { }\nexport interface MyType { }\n\n// Default export\nexport default MyComponent\n\n// Re-export from another module\nexport { something } from './other'\nexport * from './utils'\n\`\`\``;
    case 2614: {
      // Module 'X' has no exported member 'Y'
      const exportMatch = message.match(/has no exported member '(\w+)'/);
      if (exportMatch) {
        const name = exportMatch[1];
        return `**Export \`${name}\` not found** in the module.\n\n**Possible causes:**\n- Typo in import name\n- Using named import for default export\n- Export doesn't exist in this version\n\n**Fix options:**\n\`\`\`ts\n// 1. Check available exports\nimport { /* see available */ } from 'module'\n\n// 2. Maybe it's a default export?\nimport ${name} from 'module'\n\n// 3. Import all and access\nimport * as Module from 'module'\nModule.${name}\n\`\`\``;
      }
      return `**Export not found.** Check the module's available exports.`;
    }
    case 2792: // Cannot find module. Did you mean to set moduleResolution?
      return `**Module resolution configuration error.**\n\n**Fix:** Update \`tsconfig.json\`:\n\`\`\`ts\n// tsconfig.json\n{\n  "compilerOptions": {\n    // For Vite/modern bundlers\n    "moduleResolution": "bundler",\n    \n    // For Node.js ESM\n    "moduleResolution": "node16",\n    \n    // Legacy Node.js\n    "moduleResolution": "node"\n  }\n}\n\`\`\``;

    // ========== Vue Specific ==========
    case 2769: // No overload matches this call
      return `**No matching function signature.**\n\n**Why:** The arguments don't match any overload of this function.\n\n**For Vue components, check props:**\n\`\`\`ts\n// Define props with correct types\nconst props = defineProps<{\n  // Required prop\n  title: string\n  // Optional prop\n  count?: number\n  // Prop with default\n  enabled?: boolean\n}>()\n\n// Usage in parent\n<MyComponent\n  title="Hello"       // Required\n  :count="5"          // Optional number\n  :enabled="true"     // Optional boolean\n/>\n\`\`\`\n\n**For functions, check the signature:**\n\`\`\`ts\n// Multiple overloads\nfunction process(value: string): string\nfunction process(value: number): number\nfunction process(value: string | number) {\n  return value\n}\n\`\`\``;

    // ========== Misc Errors ==========
    case 1005: // ';' expected
      return `**Syntax error:** Semicolon \`;\` expected.\n\n**Common causes:**\n- Missing semicolon at end of statement\n- Unclosed bracket or parenthesis\n- Invalid syntax before this point\n\n**Fix:** Check the line above for syntax issues.`;
    case 1109: // Expression expected
      return `**Syntax error:** Expression expected.\n\n**Common causes:**\n- Incomplete statement\n- Extra comma or operator\n- Missing value in assignment\n\n**Fix:**\n\`\`\`ts\n// Wrong\nconst x = \nconst y = ,value\n\n// Correct\nconst x = value\nconst y = value\n\`\`\``;
    case 1128: // Declaration or statement expected
      return `**Syntax error:** Declaration or statement expected.\n\n**Common causes:**\n- Code outside of function/class body\n- Missing closing brace \`}\`\n- Invalid top-level code`;
    case 2365: // Operator cannot be applied
      return `**Invalid operator usage.**\n\n**Why:** This operator doesn't work with these types.\n\n**Fix:**\n\`\`\`ts\n// Wrong: comparing incompatible types\n'hello' > 5 // Error\n\n// Fix: convert to same type\nNumber('5') > 5 // OK\n'5'.localeCompare('10') // For string comparison\n\n// Wrong: arithmetic on non-numbers\n'a' + 1 // Results in 'a1' (concatenation)\n\n// Fix: ensure numeric operations\nNumber('5') + 1 // 6\n\`\`\``;
    case 2448: // Block-scoped variable already declared
      return `**Duplicate variable declaration.**\n\n**Why:** A variable with this name already exists in this scope.\n\n**Fix:**\n\`\`\`ts\n// Wrong: duplicate declaration\nconst value = 1\nconst value = 2 // Error!\n\n// Fix: use different names\nconst value = 1\nconst value2 = 2\n\n// Or reassign (with let)\nlet value = 1\nvalue = 2 // OK\n\`\`\``;
    case 2451: // Cannot redeclare block-scoped variable
      return `**Cannot redeclare variable.**\n\n**Why:** \`let\` and \`const\` create block-scoped variables that can't be redeclared.\n\n**Fix:**\n\`\`\`ts\n// Wrong\nlet value = 1\nlet value = 2 // Error!\n\n// Fix 1: Reassign instead\nlet value = 1\nvalue = 2 // OK with let\n\n// Fix 2: Use different scope\n{\n  const value = 1\n}\n{\n  const value = 2 // OK, different block\n}\n\n// Fix 3: Different variable name\nconst value1 = 1\nconst value2 = 2\n\`\`\``;
  }
  return undefined;
}

// Monaco TypeScript Worker diagnostics
interface TsDiagnostic {
  messageText: string | { messageText: string };
  message?: string;
  start: number;
  length: number;
  category: number; // 0=warning, 1=error, 2=suggestion, 3=message
  code: number;
}

// Source map entry for position mapping
interface SourceMapEntry {
  genStart: number;
  genEnd: number;
  srcStart: number;
  srcEnd: number;
}

const props = defineProps<{
  compiler: WasmModule | null;
}>();

const TYPECHECK_PRESET = `<script setup lang="ts">
import { ref } from 'vue'

// Props without type definition - triggers warning
const props = defineProps()

// Emits without type definition - triggers warning
const emit = defineEmits()

const count = ref(0)
const message = ref('Hello')

function increment() {
  count.value++
}
<\/script>

<template>
  <div class="container">
    <h1>{{ message }}</h1>
    <p>Count: {{ count }}</p>
    <button @click="increment">+1</button>
  </div>
</template>

<style scoped>
.container {
  padding: 20px;
}
</style>
`;

const TYPECHECK_TYPED_PRESET = `<script setup lang="ts">
import { ref } from 'vue'

// Props with type definition - no warning
interface Props {
  title: string
  count?: number
}
const props = defineProps<Props>()

// Emits with type definition - no warning
interface Emits {
  (e: 'update', value: number): void
  (e: 'reset'): void
}
const emit = defineEmits<Emits>()

const localCount = ref(props.count ?? 0)
const message = ref('Hello')

function increment() {
  localCount.value++
  emit('update', localCount.value)
}

function reset() {
  localCount.value = 0
  emit('reset')
}
<\/script>

<template>
  <div class="container">
    <h1>{{ props.title }}: {{ message }}</h1>
    <p>Count: {{ localCount }}</p>
    <button @click="increment">+1</button>
    <button @click="reset">Reset</button>
  </div>
</template>

<style scoped>
.container {
  padding: 20px;
}
button {
  margin: 0 4px;
}
</style>
`;

const source = ref(TYPECHECK_PRESET);
const typeCheckResult = ref<TypeCheckResult | null>(null);
const capabilities = ref<TypeCheckCapabilities | null>(null);
const error = ref<string | null>(null);
const activeTab = ref<"diagnostics" | "virtualTs" | "capabilities">(
  "diagnostics",
);
const checkTime = ref<number | null>(null);

// Options
const strictMode = ref(false);
const includeVirtualTs = ref(true); // Enable by default to show Virtual TS
const checkProps = ref(true);
const checkEmits = ref(true);
const checkTemplateBindings = ref(true);

const STORAGE_KEY = "vize-canon-typecheck-options";

// Use Monaco TypeScript for real type checking
const useMonacoTs = ref(true);
const tsDiagnostics = ref<Diagnostic[]>([]);
let virtualTsModel: monaco.editor.ITextModel | null = null;

// Configure Monaco TypeScript compiler options
async function configureTypeScript() {
  monaco.languages.typescript.typescriptDefaults.setCompilerOptions({
    target: monaco.languages.typescript.ScriptTarget.ESNext,
    module: monaco.languages.typescript.ModuleKind.ESNext,
    moduleResolution: monaco.languages.typescript.ModuleResolutionKind.NodeJs,
    strict: strictMode.value,
    noEmit: true,
    allowJs: true,
    checkJs: false,
    esModuleInterop: true,
    skipLibCheck: true,
    jsx: monaco.languages.typescript.JsxEmit.Preserve,
    noImplicitAny: false,
    strictNullChecks: strictMode.value,
  });

  // Add Vue type declarations (module + compiler macros + globals)
  monaco.languages.typescript.typescriptDefaults.addExtraLib(
    VUE_GLOBALS_DECLARATIONS,
    "vue.d.ts",
  );
}

// Vue module and type declarations for Monaco TypeScript
const VUE_GLOBALS_DECLARATIONS = `
// Vue module declaration
declare module 'vue' {
  // Reactivity: Core
  export function ref<T>(value: T): Ref<T>;
  export function ref<T = any>(): Ref<T | undefined>;
  export function reactive<T extends object>(target: T): T;
  export function readonly<T extends object>(target: T): Readonly<T>;
  export function computed<T>(getter: () => T): ComputedRef<T>;
  export function computed<T>(options: { get: () => T; set: (value: T) => void }): WritableComputedRef<T>;

  // Reactivity: Utilities
  export function unref<T>(ref: T | Ref<T>): T;
  export function toRef<T extends object, K extends keyof T>(object: T, key: K): Ref<T[K]>;
  export function toRefs<T extends object>(object: T): { [K in keyof T]: Ref<T[K]> };
  export function isRef<T>(value: Ref<T> | unknown): value is Ref<T>;
  export function isReactive(value: unknown): boolean;
  export function isReadonly(value: unknown): boolean;
  export function isProxy(value: unknown): boolean;

  // Reactivity: Advanced
  export function shallowRef<T>(value: T): ShallowRef<T>;
  export function triggerRef(ref: ShallowRef): void;
  export function customRef<T>(factory: (track: () => void, trigger: () => void) => { get: () => T; set: (value: T) => void }): Ref<T>;
  export function toRaw<T>(observed: T): T;
  export function markRaw<T extends object>(value: T): T;

  // Lifecycle Hooks
  export function onMounted(callback: () => void): void;
  export function onUnmounted(callback: () => void): void;
  export function onBeforeMount(callback: () => void): void;
  export function onBeforeUnmount(callback: () => void): void;
  export function onUpdated(callback: () => void): void;
  export function onBeforeUpdate(callback: () => void): void;
  export function onActivated(callback: () => void): void;
  export function onDeactivated(callback: () => void): void;
  export function onErrorCaptured(callback: (err: unknown, instance: any, info: string) => boolean | void): void;

  // Watch
  export function watch<T>(source: () => T, callback: (newValue: T, oldValue: T) => void, options?: WatchOptions): () => void;
  export function watch<T>(source: Ref<T>, callback: (newValue: T, oldValue: T) => void, options?: WatchOptions): () => void;
  export function watchEffect(effect: () => void, options?: WatchOptions): () => void;

  // Dependency Injection
  export function provide<T>(key: string | symbol, value: T): void;
  export function inject<T>(key: string | symbol): T | undefined;
  export function inject<T>(key: string | symbol, defaultValue: T): T;

  // Misc
  export function nextTick(callback?: () => void): Promise<void>;
  export function getCurrentInstance(): any;

  // Types
  export interface Ref<T = any> {
    value: T;
  }
  export interface ComputedRef<T = any> extends Ref<T> {
    readonly value: T;
  }
  export interface WritableComputedRef<T> extends Ref<T> {}
  export interface ShallowRef<T = any> extends Ref<T> {}
  export type UnwrapRef<T> = T extends Ref<infer V> ? V : T;
  export type Reactive<T> = T;
  export type MaybeRef<T> = T | Ref<T>;

  export interface WatchOptions {
    immediate?: boolean;
    deep?: boolean;
    flush?: 'pre' | 'post' | 'sync';
  }
}

// Vue Compiler Macros (available in <script setup>)
declare function defineProps<T>(): Readonly<T>;
declare function defineEmits<T>(): T;
declare function defineExpose<T>(exposed?: T): void;
declare function defineOptions<T>(options: T): void;
declare function defineSlots<T>(): T;
declare function defineModel<T>(name?: string, options?: { required?: boolean; default?: T }): import('vue').Ref<T>;
declare function withDefaults<T, D extends Partial<T>>(props: T, defaults: D): T & D;

// Vue Global Instance Properties (available in templates)
declare const $attrs: Record<string, unknown>;
declare const $slots: Record<string, (...args: any[]) => any>;
declare const $refs: Record<string, any>;
declare const $el: HTMLElement | undefined;
declare const $parent: any;
declare const $root: any;
declare const $emit: (...args: any[]) => void;
declare const $forceUpdate: () => void;
declare const $nextTick: (callback?: () => void) => Promise<void>;

// Event handler context
declare const $event: Event;
`;

// Cached source map entries for hover
let cachedSourceMap: SourceMapEntry[] = [];
let cachedVirtualTs: string = "";

// Virtual TS model URI - use ts-nul-authority scheme for Monaco TypeScript worker
const VIRTUAL_TS_URI = monaco.Uri.parse("ts:virtual-sfc.ts");

// Get hover info from TypeScript at a given position in Virtual TS
async function getTypeScriptHover(genOffset: number): Promise<string | null> {
  if (!virtualTsModel) return null;

  try {
    const worker = await monaco.languages.typescript.getTypeScriptWorker();
    const client = await worker(VIRTUAL_TS_URI);

    // Get quick info at position
    const quickInfo = await client.getQuickInfoAtPosition(
      VIRTUAL_TS_URI.toString(),
      genOffset,
    );
    if (!quickInfo) return null;

    // Build hover content
    const parts: string[] = [];

    if (quickInfo.displayParts) {
      const displayText = quickInfo.displayParts
        .map((p: { text: string }) => p.text)
        .join("");
      if (displayText) {
        parts.push("```typescript\n" + displayText + "\n```");
      }
    }

    if (quickInfo.documentation) {
      const docs = quickInfo.documentation
        .map((d: { text: string }) => d.text)
        .join("\n");
      if (docs) {
        parts.push(docs);
      }
    }

    return parts.length > 0 ? parts.join("\n\n") : null;
  } catch (e) {
    console.error("Failed to get TypeScript hover:", e);
    return null;
  }
}

// Map source offset to generated offset using source map
function mapSourceToGenerated(srcOffset: number): number | null {
  for (const entry of cachedSourceMap) {
    if (srcOffset >= entry.srcStart && srcOffset < entry.srcEnd) {
      // Calculate relative position within the source range
      const relativeOffset = srcOffset - entry.srcStart;
      return entry.genStart + relativeOffset;
    }
  }
  return null;
}

// Register hover provider for Vue language
let hoverProviderDisposable: monaco.IDisposable | null = null;

// Find diagnostic at a given position
function findDiagnosticAtPosition(
  line: number,
  col: number,
): Diagnostic | null {
  for (const diag of diagnostics.value) {
    const startLine = diag.startLine;
    const startCol = diag.startColumn;
    const endLine = diag.endLine ?? startLine;
    const endCol = diag.endColumn ?? startCol + 1;

    // Check if position is within diagnostic range
    if (line > startLine && line < endLine) {
      return diag;
    }
    if (line === startLine && line === endLine) {
      if (col >= startCol && col <= endCol) {
        return diag;
      }
    }
    if (line === startLine && line < endLine && col >= startCol) {
      return diag;
    }
    if (line === endLine && line > startLine && col <= endCol) {
      return diag;
    }
  }
  return null;
}

function registerHoverProvider() {
  if (hoverProviderDisposable) {
    hoverProviderDisposable.dispose();
  }

  hoverProviderDisposable = monaco.languages.registerHoverProvider("vue", {
    async provideHover(model, position) {
      const contents: monaco.IMarkdownString[] = [];

      // Check if hovering over a diagnostic
      const diag = findDiagnosticAtPosition(
        position.lineNumber,
        position.column,
      );
      if (diag) {
        // Add diagnostic message with severity indicator
        const severityLabel =
          diag.severity === "error"
            ? "Error"
            : diag.severity === "warning"
              ? "Warning"
              : "Info";
        contents.push({
          value: `**[${severityLabel}]** ${diag.message}`,
        });

        // Add help if available
        if (diag.help) {
          contents.push({
            value: `---\n**Hint**\n\n${diag.help}`,
          });
        }
      }

      // Also get TypeScript type info
      const srcOffset = model.getOffsetAt(position);
      const genOffset = mapSourceToGenerated(srcOffset);
      if (genOffset !== null) {
        const hoverContent = await getTypeScriptHover(genOffset);
        if (hoverContent) {
          if (contents.length > 0) {
            contents.push({ value: "---" });
          }
          contents.push({ value: hoverContent });
        }
      }

      if (contents.length === 0) return null;

      // Return hover info
      return {
        contents,
      };
    },
  });
}

// Get TypeScript diagnostics from Monaco Worker
async function getTypeScriptDiagnostics(
  virtualTs: string,
): Promise<Diagnostic[]> {
  if (!virtualTs) return [];

  // Create or update the virtual TS model
  if (virtualTsModel) {
    virtualTsModel.setValue(virtualTs);
  } else {
    virtualTsModel = monaco.editor.createModel(
      virtualTs,
      "typescript",
      VIRTUAL_TS_URI,
    );
  }

  try {
    // Get TypeScript Worker
    const worker = await monaco.languages.typescript.getTypeScriptWorker();
    const client = await worker(VIRTUAL_TS_URI);

    // Get semantic and syntactic diagnostics
    const [semanticDiags, syntacticDiags] = await Promise.all([
      client.getSemanticDiagnostics(VIRTUAL_TS_URI.toString()),
      client.getSyntacticDiagnostics(VIRTUAL_TS_URI.toString()),
    ]);

    const allDiags = [...syntacticDiags, ...semanticDiags] as TsDiagnostic[];

    console.log(
      "[TypeCheck] Virtual TS diagnostics:",
      allDiags.length,
      JSON.stringify(allDiags, null, 2),
    );

    // Convert to our Diagnostic format
    return allDiags.map((d) => {
      const startPos = virtualTsModel!.getPositionAt(d.start);
      const endPos = virtualTsModel!.getPositionAt(d.start + d.length);

      // Extract message text - can be string, object with messageText, or nested chain
      let message = "Unknown error";
      if (typeof d.messageText === "string") {
        message = d.messageText;
      } else if (d.messageText && typeof d.messageText === "object") {
        // DiagnosticMessageChain - get the first message
        message = (d.messageText as any).messageText || "Unknown error";
      } else if (typeof d.message === "string") {
        message = d.message;
      }

      // TypeScript DiagnosticCategory: 0=Warning, 1=Error, 2=Suggestion, 3=Message
      const severity =
        d.category === 1 ? "error" : d.category === 0 ? "warning" : "info";

      return {
        message,
        code: d.code, // Preserve error code for help generation
        startLine: startPos.lineNumber,
        startColumn: startPos.column,
        endLine: endPos.lineNumber,
        endColumn: endPos.column,
        severity: severity as "error" | "warning" | "info",
      };
    });
  } catch (e) {
    console.error("Failed to get TypeScript diagnostics:", e);
    return [];
  }
}

// Parse source map from generated Virtual TS
function parseSourceMap(virtualTs: string): SourceMapEntry[] {
  const entries: SourceMapEntry[] = [];

  // Look for source map markers in comments
  // Format: // @vize-map: genStart:genEnd -> srcStart:srcEnd
  const regex = /\/\/ @vize-map:\s*(\d+):(\d+)\s*->\s*(\d+):(\d+)/g;
  let match;
  while ((match = regex.exec(virtualTs)) !== null) {
    entries.push({
      genStart: parseInt(match[1]),
      genEnd: parseInt(match[2]),
      srcStart: parseInt(match[3]),
      srcEnd: parseInt(match[4]),
    });
  }

  return entries;
}

// Map diagnostics from Virtual TS to original Vue source
function mapDiagnosticsToSource(
  tsDiags: Diagnostic[],
  virtualTs: string,
  vueSource: string,
): Diagnostic[] {
  // Parse source map entries from Virtual TS comments
  const sourceMapEntries = parseSourceMap(virtualTs);

  const mapped: Diagnostic[] = [];

  // Helper: convert line/column to offset
  function lineColToOffset(content: string, line: number, col: number): number {
    const lines = content.split("\n");
    let offset = 0;
    for (let i = 0; i < line - 1 && i < lines.length; i++) {
      offset += lines[i].length + 1; // +1 for newline
    }
    return offset + col - 1;
  }

  // Helper: convert offset to line/column in Vue source
  function offsetToLineCol(
    content: string,
    offset: number,
  ): { line: number; col: number } {
    const lines = content.split("\n");
    let currentOffset = 0;
    for (let i = 0; i < lines.length; i++) {
      const lineEnd = currentOffset + lines[i].length + 1;
      if (offset < lineEnd) {
        return { line: i + 1, col: offset - currentOffset + 1 };
      }
      currentOffset = lineEnd;
    }
    return { line: lines.length, col: 1 };
  }

  for (const diag of tsDiags) {
    // Calculate offset in virtual TS
    const diagOffset = lineColToOffset(
      virtualTs,
      diag.startLine,
      diag.startColumn,
    );
    const diagEndOffset = lineColToOffset(
      virtualTs,
      diag.endLine || diag.startLine,
      diag.endColumn || diag.startColumn,
    );

    // Try to find a matching source map entry
    let foundMapping = false;
    for (const entry of sourceMapEntries) {
      if (diagOffset >= entry.genStart && diagOffset <= entry.genEnd) {
        // Calculate relative position within the generated range
        const relativeOffset = diagOffset - entry.genStart;
        const srcOffset = entry.srcStart + relativeOffset;
        const srcEndOffset = Math.min(
          entry.srcEnd,
          srcOffset + (diagEndOffset - diagOffset),
        );

        const startPos = offsetToLineCol(vueSource, srcOffset);
        const endPos = offsetToLineCol(vueSource, srcEndOffset);

        // Generate help suggestion based on error code
        const help = diag.code
          ? generateHelp(diag.code, diag.message)
          : undefined;

        mapped.push({
          ...diag,
          startLine: startPos.line,
          startColumn: startPos.col,
          endLine: endPos.line,
          endColumn: endPos.col,
          message: diag.code
            ? `[vize:TS${diag.code}] ${diag.message}`
            : `[vize] ${diag.message}`,
          help,
        });
        foundMapping = true;
        break;
      }
    }

    // Only include diagnostics that have valid source mappings
    // Skip diagnostics from Virtual TS boilerplate (no mapping = generated code)
    if (!foundMapping) {
      // Skip errors from boilerplate - these are not user code errors
      console.log("[TypeCheck] Skipping unmapped diagnostic:", diag.message);
    }
  }

  return mapped;
}

// Load saved options from localStorage
function loadOptions() {
  try {
    const saved = localStorage.getItem(STORAGE_KEY);
    if (saved) {
      const config = JSON.parse(saved);
      strictMode.value = config.strictMode ?? false;
      includeVirtualTs.value = config.includeVirtualTs ?? true; // Default to true
      checkProps.value = config.checkProps ?? true;
      checkEmits.value = config.checkEmits ?? true;
      checkTemplateBindings.value = config.checkTemplateBindings ?? true;
      useMonacoTs.value = config.useMonacoTs ?? true; // Default to true
    }
  } catch (e) {
    console.warn("Failed to load options:", e);
  }
}

// Save options to localStorage
function saveOptions() {
  try {
    const config = {
      strictMode: strictMode.value,
      includeVirtualTs: includeVirtualTs.value,
      checkProps: checkProps.value,
      checkEmits: checkEmits.value,
      checkTemplateBindings: checkTemplateBindings.value,
      useMonacoTs: useMonacoTs.value,
    };
    localStorage.setItem(STORAGE_KEY, JSON.stringify(config));
  } catch (e) {
    console.warn("Failed to save options:", e);
  }
}

const errorCount = computed(() => {
  const wasmErrors = typeCheckResult.value?.errorCount ?? 0;
  const tsErrors = tsDiagnostics.value.filter(
    (d) => d.severity === "error",
  ).length;
  return wasmErrors + tsErrors;
});
const warningCount = computed(() => {
  const wasmWarnings = typeCheckResult.value?.warningCount ?? 0;
  const tsWarnings = tsDiagnostics.value.filter(
    (d) => d.severity === "warning",
  ).length;
  return wasmWarnings + tsWarnings;
});

// Calculate position from offset
function getPositionFromOffset(
  source: string,
  offset: number,
): { line: number; column: number } {
  const lines = source.substring(0, offset).split("\n");
  return {
    line: lines.length,
    column: lines[lines.length - 1].length + 1,
  };
}

// Convert type check diagnostics to Monaco markers (combining WASM and TS Worker diagnostics)
const diagnostics = computed((): Diagnostic[] => {
  const wasmDiags: Diagnostic[] = [];

  // Add WASM-based diagnostics (from vize static analysis)
  if (typeCheckResult.value?.diagnostics) {
    for (const d of typeCheckResult.value.diagnostics) {
      const startPos = getPositionFromOffset(source.value, d.start);
      const endPos = getPositionFromOffset(source.value, d.end);
      // Format: [vize:CODE] message
      const message = d.code
        ? `[vize:${d.code}] ${d.message}`
        : `[vize] ${d.message}`;
      wasmDiags.push({
        message,
        help: d.help, // WASM diagnostics may include help from Rust side
        startLine: startPos.line,
        startColumn: startPos.column,
        endLine: endPos.line,
        endColumn: endPos.column,
        severity:
          d.severity === "error"
            ? "error"
            : d.severity === "warning"
              ? "warning"
              : "info",
      });
    }
  }

  // Add TypeScript Worker diagnostics
  if (useMonacoTs.value) {
    return [...wasmDiags, ...tsDiagnostics.value];
  }

  return wasmDiags;
});

async function typeCheck() {
  if (!props.compiler) return;

  const startTime = performance.now();
  error.value = null;

  try {
    const result = props.compiler.typeCheck(source.value, {
      filename: "example.vue",
      strict: strictMode.value,
      includeVirtualTs: true, // Always generate virtual TS for Monaco checking
      checkProps: checkProps.value,
      checkEmits: checkEmits.value,
      checkTemplateBindings: checkTemplateBindings.value,
    });
    typeCheckResult.value = result;

    // If Monaco TS checking is enabled and we have virtual TS
    if (useMonacoTs.value && result.virtualTs) {
      // Cache source map for hover
      cachedVirtualTs = result.virtualTs;
      cachedSourceMap = parseSourceMap(result.virtualTs);

      const tsDiags = await getTypeScriptDiagnostics(result.virtualTs);
      tsDiagnostics.value = mapDiagnosticsToSource(
        tsDiags,
        result.virtualTs,
        source.value,
      );
    } else {
      tsDiagnostics.value = [];
      cachedSourceMap = [];
      cachedVirtualTs = "";
    }

    checkTime.value = performance.now() - startTime;
  } catch (e) {
    error.value = e instanceof Error ? e.message : String(e);
    typeCheckResult.value = null;
    tsDiagnostics.value = [];
  }
}

function loadCapabilities() {
  if (!props.compiler) return;

  try {
    capabilities.value = props.compiler.getTypeCheckCapabilities();
  } catch (e) {
    console.error("Failed to load capabilities:", e);
  }
}

// Simple syntax highlighter for code - uses token-based approach to avoid conflicts
function highlightCode(code: string, lang: string): string {
  // Token placeholders to prevent regex conflicts
  const tokens: string[] = [];
  let tokenId = 0;
  const placeholder = (content: string): string => {
    const id = `__TOKEN_${tokenId++}__`;
    tokens.push(content);
    return id;
  };

  let result = code;

  // Vue/HTML specific
  if (lang === "vue" || lang === "html") {
    // HTML comments first
    result = result.replace(/(&lt;!--[\s\S]*?--&gt;)/g, (_, m) =>
      placeholder(`<span class="hl-comment">${m}</span>`),
    );
    // Attribute values in quotes (before tags to avoid conflicts)
    result = result.replace(
      /="([^"]*)"/g,
      (_, v) => `="${placeholder(`<span class="hl-string">${v}</span>`)}"`,
    );
    // Vue directives
    result = result.replace(
      /(v-[\w-]+|@[\w.-]+|:[\w.-]+(?==")|#[\w.-]+)/g,
      (_, m) => placeholder(`<span class="hl-directive">${m}</span>`),
    );
    // Tags (opening and closing)
    result = result.replace(
      /(&lt;\/?)([\w-]+)/g,
      (_, prefix, tag) =>
        `${prefix}${placeholder(`<span class="hl-tag">${tag}</span>`)}`,
    );
    // Mustache interpolation
    result = result.replace(/(\{\{|\}\})/g, (_, m) =>
      placeholder(`<span class="hl-delimiter">${m}</span>`),
    );
  }

  // TypeScript/JavaScript
  if (
    lang === "ts" ||
    lang === "typescript" ||
    lang === "js" ||
    lang === "javascript"
  ) {
    // Comments first (to avoid highlighting inside comments)
    result = result.replace(/(\/\/.*)/g, (_, m) =>
      placeholder(`<span class="hl-comment">${m}</span>`),
    );
    // Strings (must be before keywords to avoid highlighting keywords inside strings)
    result = result.replace(/('[^']*'|"[^"]*"|`[^`]*`)/g, (_, m) =>
      placeholder(`<span class="hl-string">${m}</span>`),
    );
    // Vue APIs (before general keywords)
    result = result.replace(
      /\b(ref|reactive|computed|watch|watchEffect|onMounted|onUnmounted|defineProps|defineEmits|toRefs|inject|provide)\b/g,
      (_, m) => placeholder(`<span class="hl-vue-api">${m}</span>`),
    );
    // Keywords
    result = result.replace(
      /\b(const|let|var|function|return|if|else|for|while|import|export|from|async|await|new|typeof|instanceof|class|interface|type|extends)\b/g,
      (_, m) => placeholder(`<span class="hl-keyword">${m}</span>`),
    );
    // Types
    result = result.replace(
      /\b(string|number|boolean|null|undefined|void|any|never)\b/g,
      (_, m) => placeholder(`<span class="hl-type">${m}</span>`),
    );
    // Numbers
    result = result.replace(/\b(\d+)\b/g, (_, m) =>
      placeholder(`<span class="hl-number">${m}</span>`),
    );
  }

  // CSS
  if (lang === "css") {
    // At-rules
    result = result.replace(/(@[\w-]+)/g, (_, m) =>
      placeholder(`<span class="hl-keyword">${m}</span>`),
    );
    // Properties
    result = result.replace(
      /([\w-]+)(\s*:)/g,
      (_, prop, colon) =>
        `${placeholder(`<span class="hl-property">${prop}</span>`)}${colon}`,
    );
  }

  // Bash
  if (lang === "bash" || lang === "sh") {
    // Comments first
    result = result.replace(/(#.*)/g, (_, m) =>
      placeholder(`<span class="hl-comment">${m}</span>`),
    );
    // Commands
    result = result.replace(
      /\b(npm|yarn|pnpm|git|cd|mkdir|rm|cp|mv|install)\b/g,
      (_, m) => placeholder(`<span class="hl-keyword">${m}</span>`),
    );
  }

  // Replace all token placeholders with actual content
  for (let i = 0; i < tokens.length; i++) {
    result = result.replace(`__TOKEN_${i}__`, tokens[i]);
  }

  return result;
}

// Simple markdown formatter for help text
function formatHelp(help: string): string {
  let result = help
    // Escape HTML first
    .replace(/&/g, "&amp;")
    .replace(/</g, "&lt;")
    .replace(/>/g, "&gt;");

  // Code blocks (```lang ... ```)
  result = result.replace(/```(\w*)\n([\s\S]*?)```/g, (_, lang, code) => {
    const highlighted = highlightCode(code, lang || "text");
    return `<pre class="help-code" data-lang="${lang || "text"}"><code>${highlighted}</code></pre>`;
  });

  // Inline code (`code`)
  result = result.replace(
    /`([^`]+)`/g,
    '<code class="help-inline-code">$1</code>',
  );
  // Bold (**text**)
  result = result.replace(/\*\*([^*]+)\*\*/g, "<strong>$1</strong>");
  // Line breaks
  result = result.replace(/\n/g, "<br>");

  return result;
}

// Format diagnostic message with basic markdown (inline code, types)
function formatMessage(message: string): string {
  return (
    message
      // Escape HTML first
      .replace(/&/g, "&amp;")
      .replace(/</g, "&lt;")
      .replace(/>/g, "&gt;")
      // Highlight type names in quotes: 'TypeName' or "TypeName"
      .replace(/'([^']+)'/g, '<code class="msg-type">$1</code>')
      .replace(/"([^"]+)"/g, '<code class="msg-type">$1</code>')
  );
}

function getSeverityIcon(
  severity: "error" | "warning" | "info" | "hint",
): string {
  switch (severity) {
    case "error":
      return "\u2717";
    case "warning":
      return "\u26A0";
    case "info":
      return "\u24D8";
    default:
      return "\u2022";
  }
}

function setPreset(preset: "untyped" | "typed") {
  source.value = preset === "typed" ? TYPECHECK_TYPED_PRESET : TYPECHECK_PRESET;
}

let checkTimer: ReturnType<typeof setTimeout> | null = null;

watch(
  source,
  () => {
    if (checkTimer) clearTimeout(checkTimer);
    checkTimer = setTimeout(typeCheck, 300);
  },
  { immediate: true },
);

watch(
  [
    strictMode,
    includeVirtualTs,
    checkProps,
    checkEmits,
    checkTemplateBindings,
    useMonacoTs,
  ],
  () => {
    saveOptions();
    typeCheck();
  },
);

watch(
  () => props.compiler,
  () => {
    if (props.compiler) {
      typeCheck();
      loadCapabilities();
    }
  },
);

onMounted(async () => {
  loadOptions();
  await configureTypeScript();
  registerHoverProvider();
  if (props.compiler) {
    loadCapabilities();
  }
});

onUnmounted(() => {
  // Clean up the virtual TS model
  if (virtualTsModel) {
    virtualTsModel.dispose();
    virtualTsModel = null;
  }
  // Clean up hover provider
  if (hoverProviderDisposable) {
    hoverProviderDisposable.dispose();
    hoverProviderDisposable = null;
  }
});
</script>

<template>
  <div class="typecheck-playground">
    <div class="panel input-panel">
      <div class="panel-header">
        <div class="header-title">
          <span class="icon">&lt;/&gt;</span>
          <h2>Source</h2>
        </div>
        <div class="panel-actions">
          <button @click="setPreset('untyped')" class="btn-ghost">
            Untyped
          </button>
          <button @click="setPreset('typed')" class="btn-ghost">Typed</button>
        </div>
      </div>
      <div class="editor-container">
        <MonacoEditor
          v-model="source"
          language="vue"
          :diagnostics="diagnostics"
        />
      </div>
    </div>

    <div class="panel output-panel">
      <div class="panel-header">
        <div class="header-title">
          <span class="icon">&#x2714;</span>
          <h2>Type Analysis</h2>
          <span v-if="checkTime !== null" class="perf-badge">
            {{ checkTime.toFixed(2) }}ms
          </span>
          <template v-if="typeCheckResult">
            <span v-if="errorCount > 0" class="count-badge errors">{{
              errorCount
            }}</span>
            <span v-if="warningCount > 0" class="count-badge warnings">{{
              warningCount
            }}</span>
          </template>
        </div>
        <div class="tabs">
          <button
            :class="['tab', { active: activeTab === 'diagnostics' }]"
            @click="activeTab = 'diagnostics'"
          >
            Diagnostics
            <span v-if="diagnostics.length" class="tab-badge">{{
              diagnostics.length
            }}</span>
          </button>
          <button
            :class="['tab', { active: activeTab === 'virtualTs' }]"
            @click="activeTab = 'virtualTs'"
          >
            Virtual TS
          </button>
          <button
            :class="['tab', { active: activeTab === 'capabilities' }]"
            @click="activeTab = 'capabilities'"
          >
            Info
          </button>
        </div>
      </div>

      <div class="output-content">
        <div v-if="error" class="error-panel">
          <div class="error-header">Type Check Error</div>
          <pre class="error-content">{{ error }}</pre>
        </div>

        <template v-else-if="typeCheckResult">
          <!-- Diagnostics Tab -->
          <div v-if="activeTab === 'diagnostics'" class="diagnostics-output">
            <div class="output-header-bar">
              <span class="output-title">Type Issues</span>
              <div class="options-toggle">
                <label class="option-label">
                  <input type="checkbox" v-model="strictMode" />
                  Strict
                </label>
              </div>
            </div>

            <div class="options-panel">
              <label class="option-label highlight">
                <input type="checkbox" v-model="useMonacoTs" />
                TypeScript (Monaco)
              </label>
              <label class="option-label">
                <input type="checkbox" v-model="checkProps" />
                Check Props
              </label>
              <label class="option-label">
                <input type="checkbox" v-model="checkEmits" />
                Check Emits
              </label>
              <label class="option-label">
                <input type="checkbox" v-model="checkTemplateBindings" />
                Check Template Bindings
              </label>
              <label class="option-label">
                <input type="checkbox" v-model="includeVirtualTs" />
                Show Virtual TS
              </label>
            </div>

            <div v-if="diagnostics.length === 0" class="success-state">
              <span class="success-icon">&#x2713;</span>
              <span>No type issues found</span>
            </div>

            <div v-else class="diagnostics-list">
              <div
                v-for="(diagnostic, i) in diagnostics"
                :key="i"
                :class="['diagnostic-item', `severity-${diagnostic.severity}`]"
              >
                <div class="diagnostic-header">
                  <span class="severity-icon">{{
                    getSeverityIcon(diagnostic.severity)
                  }}</span>
                  <code v-if="diagnostic.code" class="error-code"
                    >TS{{ diagnostic.code }}</code
                  >
                  <span class="location-badge">
                    {{ diagnostic.startLine }}:{{ diagnostic.startColumn }}
                  </span>
                </div>
                <div
                  class="diagnostic-message"
                  v-html="formatMessage(diagnostic.message)"
                ></div>
                <div v-if="diagnostic.help" class="diagnostic-help">
                  <div class="help-header">
                    <span class="help-icon">?</span>
                    <span class="help-label">Hint</span>
                  </div>
                  <div
                    class="help-content"
                    v-html="formatHelp(diagnostic.help)"
                  ></div>
                </div>
              </div>
            </div>
          </div>

          <!-- Virtual TS Tab -->
          <div v-else-if="activeTab === 'virtualTs'" class="virtualts-output">
            <div class="output-header-bar">
              <span class="output-title">Generated TypeScript</span>
            </div>
            <div v-if="typeCheckResult.virtualTs" class="editor-container">
              <MonacoEditor
                :model-value="typeCheckResult.virtualTs"
                language="typescript"
                :read-only="true"
              />
            </div>
            <div v-else class="empty-state">
              <span
                >Enable "Generate Virtual TS" option to see generated
                TypeScript</span
              >
            </div>
          </div>

          <!-- Capabilities Tab -->
          <div
            v-else-if="activeTab === 'capabilities'"
            class="capabilities-output"
          >
            <div class="output-header-bar">
              <span class="output-title">Type Checker Capabilities</span>
            </div>

            <div v-if="capabilities" class="capabilities-content">
              <div class="capability-section">
                <h3>Mode</h3>
                <code class="mode-badge">{{ capabilities.mode }}</code>
                <p>{{ capabilities.description }}</p>
              </div>

              <div class="capability-section">
                <h3>Available Checks</h3>
                <div class="checks-list">
                  <div
                    v-for="check in capabilities.checks"
                    :key="check.name"
                    class="check-item"
                  >
                    <code class="check-name">{{ check.name }}</code>
                    <span :class="['check-severity', check.severity]">{{
                      check.severity
                    }}</span>
                    <p class="check-description">{{ check.description }}</p>
                  </div>
                </div>
              </div>

              <div class="capability-section">
                <h3>Notes</h3>
                <ul class="notes-list">
                  <li v-for="(note, i) in capabilities.notes" :key="i">
                    {{ note }}
                  </li>
                </ul>
              </div>
            </div>
          </div>
        </template>

        <div v-else class="loading-state">
          <span>Enter Vue code to see type analysis</span>
        </div>
      </div>
    </div>
  </div>
</template>

<style scoped>
.typecheck-playground {
  display: grid;
  grid-template-columns: 1fr 1fr;
  gap: 0;
  height: 100%;
  min-height: 0;
  grid-column: 1 / -1;
  background: var(--bg-primary);
}

.panel {
  display: flex;
  flex-direction: column;
  overflow: hidden;
  min-height: 0;
}

.input-panel {
  border-right: 1px solid var(--border-primary);
}

.panel-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 0.75rem 1rem;
  background: var(--bg-secondary);
  border-bottom: 1px solid var(--border-primary);
  flex-shrink: 0;
}

.header-title {
  display: flex;
  align-items: center;
  gap: 0.5rem;
}

.header-title .icon {
  font-size: 1rem;
  color: var(--accent-blue);
}

.header-title h2 {
  font-size: 0.875rem;
  font-weight: 600;
  margin: 0;
}

.perf-badge {
  font-size: 0.625rem;
  padding: 0.125rem 0.375rem;
  background: rgba(74, 222, 128, 0.15);
  color: #4ade80;
  border-radius: 3px;
  font-family: "JetBrains Mono", monospace;
}

.count-badge {
  font-size: 0.625rem;
  padding: 0.0625rem 0.375rem;
  border-radius: 8px;
  min-width: 1.25rem;
  text-align: center;
  font-family: "JetBrains Mono", monospace;
}

.count-badge.errors {
  background: rgba(239, 68, 68, 0.2);
  color: #f87171;
}

.count-badge.warnings {
  background: rgba(245, 158, 11, 0.2);
  color: #fbbf24;
}

.panel-actions {
  display: flex;
  gap: 0.5rem;
}

.btn-ghost {
  padding: 0.25rem 0.5rem;
  font-size: 0.75rem;
  background: transparent;
  border: 1px solid var(--border-primary);
  border-radius: 4px;
  color: var(--text-secondary);
  cursor: pointer;
  transition: all 0.15s;
}

.btn-ghost:hover {
  background: var(--bg-tertiary);
  color: var(--text-primary);
}

.tabs {
  display: flex;
  gap: 0.125rem;
}

.tab {
  padding: 0.375rem 0.625rem;
  font-size: 0.75rem;
  background: transparent;
  border: none;
  border-radius: 4px;
  color: var(--text-muted);
  cursor: pointer;
  transition: all 0.15s;
  display: flex;
  align-items: center;
  gap: 0.375rem;
}

.tab:hover {
  color: var(--text-secondary);
  background: var(--bg-tertiary);
}

.tab.active {
  color: var(--text-primary);
  background: var(--bg-tertiary);
  font-weight: 500;
}

.tab-badge {
  font-size: 0.625rem;
  padding: 0.0625rem 0.3125rem;
  background: rgba(239, 68, 68, 0.2);
  color: #f87171;
  border-radius: 8px;
  min-width: 1rem;
  text-align: center;
}

.editor-container {
  flex: 1;
  overflow: hidden;
}

.output-content {
  flex: 1;
  overflow-y: auto;
  padding: 1rem;
}

/* Error State */
.error-panel {
  background: rgba(239, 68, 68, 0.1);
  border: 1px solid rgba(239, 68, 68, 0.3);
  border-radius: 6px;
  overflow: hidden;
}

.error-header {
  padding: 0.5rem 0.75rem;
  background: rgba(239, 68, 68, 0.15);
  color: #f87171;
  font-size: 0.75rem;
  font-weight: 600;
}

.error-content {
  padding: 0.75rem;
  font-size: 0.75rem;
  color: #fca5a5;
  margin: 0;
  white-space: pre-wrap;
  word-break: break-word;
}

/* Output Header Bar */
.output-header-bar {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 0.5rem 0.75rem;
  background: linear-gradient(
    135deg,
    rgba(59, 130, 246, 0.15),
    rgba(139, 92, 246, 0.15)
  );
  border: 1px solid rgba(59, 130, 246, 0.3);
  border-radius: 4px;
  margin-bottom: 0.75rem;
}

.output-title {
  font-size: 0.75rem;
  font-weight: 600;
  color: #60a5fa;
  text-transform: uppercase;
  letter-spacing: 0.05em;
}

/* Options */
.options-panel {
  display: flex;
  flex-wrap: wrap;
  gap: 0.75rem;
  padding: 0.5rem 0.75rem;
  background: var(--bg-secondary);
  border: 1px solid var(--border-primary);
  border-radius: 4px;
  margin-bottom: 0.75rem;
}

.option-label {
  display: flex;
  align-items: center;
  gap: 0.375rem;
  font-size: 0.6875rem;
  color: var(--text-secondary);
  cursor: pointer;
}

.option-label input[type="checkbox"] {
  width: 12px;
  height: 12px;
  accent-color: var(--accent-blue);
}

.option-label.highlight {
  background: rgba(59, 130, 246, 0.15);
  padding: 0.25rem 0.5rem;
  border-radius: 4px;
  border: 1px solid rgba(59, 130, 246, 0.3);
}

/* Success State */
.success-state {
  display: flex;
  align-items: center;
  justify-content: center;
  gap: 0.5rem;
  padding: 2rem;
  color: #4ade80;
  font-size: 0.875rem;
}

.success-icon {
  font-size: 1.25rem;
}

/* Diagnostics List */
.diagnostics-list {
  display: flex;
  flex-direction: column;
  gap: 0.5rem;
}

.diagnostic-item {
  padding: 0.75rem;
  background: var(--bg-secondary);
  border: 1px solid var(--border-primary);
  border-radius: 4px;
  border-left: 3px solid;
}

.diagnostic-item.severity-error {
  border-left-color: #ef4444;
}

.diagnostic-item.severity-warning {
  border-left-color: #f59e0b;
}

.diagnostic-item.severity-info {
  border-left-color: #60a5fa;
}

.diagnostic-header {
  display: flex;
  align-items: center;
  gap: 0.5rem;
  margin-bottom: 0.375rem;
}

.severity-icon {
  font-size: 0.75rem;
  font-weight: bold;
}

.severity-error .severity-icon {
  color: #ef4444;
}

.severity-warning .severity-icon {
  color: #f59e0b;
}

.severity-info .severity-icon {
  color: #60a5fa;
}

.error-code {
  font-size: 0.6875rem;
  font-family: "JetBrains Mono", monospace;
  color: var(--text-muted);
  background: var(--bg-tertiary);
  padding: 0.125rem 0.375rem;
  border-radius: 3px;
}

.location-badge {
  margin-left: auto;
  font-size: 0.625rem;
  font-family: "JetBrains Mono", monospace;
  color: var(--text-muted);
}

.diagnostic-message {
  font-size: 0.8125rem;
  color: var(--text-primary);
  line-height: 1.4;
}

.diagnostic-message :deep(.msg-type) {
  font-family: "JetBrains Mono", monospace;
  font-size: 0.85em;
  color: #79c0ff;
  background: rgba(121, 192, 255, 0.1);
  padding: 0.1rem 0.3rem;
  border-radius: 3px;
}

.diagnostic-help {
  margin-top: 0.75rem;
  padding: 0.75rem;
  background: linear-gradient(
    135deg,
    rgba(96, 165, 250, 0.08) 0%,
    rgba(147, 51, 234, 0.05) 100%
  );
  border: 1px solid rgba(96, 165, 250, 0.2);
  border-radius: 6px;
  font-size: 0.85rem;
}

.help-header {
  display: flex;
  align-items: center;
  gap: 0.5rem;
  margin-bottom: 0.5rem;
  padding-bottom: 0.5rem;
  border-bottom: 1px solid rgba(96, 165, 250, 0.15);
}

.help-icon {
  font-size: 1rem;
}

.help-label {
  font-weight: 600;
  color: #60a5fa;
  font-size: 0.9rem;
}

.help-content {
  color: var(--text-primary);
  line-height: 1.6;
}

.help-content :deep(strong) {
  color: #f59e0b;
  font-weight: 600;
}

.help-content :deep(.help-code) {
  margin: 0.5rem 0;
  padding: 0.75rem;
  background: rgba(0, 0, 0, 0.3);
  border-radius: 4px;
  overflow-x: auto;
  font-family: "JetBrains Mono", "Fira Code", monospace;
  font-size: 0.8rem;
  line-height: 1.5;
}

.help-content :deep(.help-code code) {
  color: #a5d6ff;
  background: none;
  padding: 0;
}

.help-content :deep(.help-inline-code) {
  background: rgba(110, 118, 129, 0.3);
  padding: 0.15rem 0.4rem;
  border-radius: 3px;
  font-family: "JetBrains Mono", "Fira Code", monospace;
  font-size: 0.85em;
  color: #ff7b72;
}

/* Syntax highlighting colors */
.help-content :deep(.hl-keyword) {
  color: #ff7b72;
}

.help-content :deep(.hl-vue-api) {
  color: #7ee787;
}

.help-content :deep(.hl-string) {
  color: #a5d6ff;
}

.help-content :deep(.hl-comment) {
  color: #8b949e;
  font-style: italic;
}

.help-content :deep(.hl-tag) {
  color: #7ee787;
}

.help-content :deep(.hl-directive) {
  color: #d2a8ff;
}

.help-content :deep(.hl-delimiter) {
  color: #ffa657;
}

.help-content :deep(.hl-type) {
  color: #79c0ff;
}

.help-content :deep(.hl-number) {
  color: #79c0ff;
}

.help-content :deep(.hl-property) {
  color: #79c0ff;
}

.help-content :deep(.hl-value) {
  color: #a5d6ff;
}

.severity-error .severity-icon {
  color: #ef4444;
}

.severity-warning .severity-icon {
  color: #f59e0b;
}

.severity-info .severity-icon {
  color: #60a5fa;
}

.diagnostic-code {
  font-size: 0.6875rem;
  font-family: "JetBrains Mono", monospace;
  color: var(--text-muted);
  background: var(--bg-tertiary);
  padding: 0.125rem 0.375rem;
  border-radius: 3px;
}

.location-badge {
  margin-left: auto;
  font-size: 0.625rem;
  font-family: "JetBrains Mono", monospace;
  color: var(--text-muted);
}

/* Virtual TS Output */
.virtualts-output {
  display: flex;
  flex-direction: column;
  height: 100%;
}

.virtualts-output .editor-container {
  flex: 1;
  min-height: 200px;
}

/* Capabilities Output */
.capabilities-content {
  display: flex;
  flex-direction: column;
  gap: 1rem;
}

.capability-section h3 {
  font-size: 0.75rem;
  font-weight: 600;
  color: var(--text-secondary);
  margin: 0 0 0.5rem 0;
  text-transform: uppercase;
  letter-spacing: 0.05em;
}

.capability-section p {
  font-size: 0.8125rem;
  color: var(--text-muted);
  margin: 0.25rem 0 0 0;
}

.mode-badge {
  display: inline-block;
  padding: 0.25rem 0.5rem;
  background: var(--bg-tertiary);
  border: 1px solid var(--border-primary);
  border-radius: 4px;
  font-size: 0.75rem;
  color: #60a5fa;
}

.checks-list {
  display: flex;
  flex-direction: column;
  gap: 0.5rem;
}

.check-item {
  padding: 0.5rem 0.75rem;
  background: var(--bg-secondary);
  border: 1px solid var(--border-primary);
  border-radius: 4px;
}

.check-name {
  font-size: 0.75rem;
  font-family: "JetBrains Mono", monospace;
  color: var(--text-primary);
}

.check-severity {
  margin-left: 0.5rem;
  font-size: 0.625rem;
  padding: 0.125rem 0.375rem;
  border-radius: 3px;
  text-transform: uppercase;
}

.check-severity.error {
  background: rgba(239, 68, 68, 0.15);
  color: #f87171;
}

.check-severity.warning {
  background: rgba(245, 158, 11, 0.15);
  color: #fbbf24;
}

.check-description {
  font-size: 0.6875rem;
  color: var(--text-muted);
  margin: 0.25rem 0 0 0;
}

.notes-list {
  margin: 0;
  padding-left: 1rem;
  color: var(--text-muted);
  font-size: 0.75rem;
}

.notes-list li {
  margin: 0.25rem 0;
}

.empty-state,
.loading-state {
  display: flex;
  align-items: center;
  justify-content: center;
  padding: 2rem;
  color: var(--text-muted);
  font-size: 0.875rem;
}

/* Mobile responsive */
@media (max-width: 768px) {
  .typecheck-playground {
    grid-template-columns: 1fr;
    grid-template-rows: minmax(300px, 1fr) minmax(300px, 1fr);
    height: auto;
    min-height: 100%;
  }

  .panel {
    min-height: 300px;
  }

  .input-panel {
    border-right: none;
    border-bottom: 1px solid var(--border-primary);
  }

  .panel-header {
    flex-wrap: wrap;
    gap: 0.5rem;
  }

  .tabs {
    flex-wrap: wrap;
    width: 100%;
  }

  .options-panel {
    flex-direction: column;
  }
}
</style>
