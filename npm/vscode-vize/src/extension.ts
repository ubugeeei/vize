import * as path from "path";
import * as fs from "fs";
import { ExtensionContext, commands, window, workspace, OutputChannel } from "vscode";
import {
  LanguageClient,
  LanguageClientOptions,
  ServerOptions,
  TransportKind,
  Executable,
} from "vscode-languageclient/node";

let client: LanguageClient | undefined;
let outputChannel: OutputChannel;

export async function activate(context: ExtensionContext): Promise<void> {
  outputChannel = window.createOutputChannel("Vize");
  outputChannel.appendLine("Vize extension activating...");

  const config = workspace.getConfiguration("vize");
  if (!config.get<boolean>("enable", true)) {
    outputChannel.appendLine("Vize is disabled in settings");
    return;
  }

  // Find the language server executable
  const serverPath = await findServerPath(context, config);
  if (!serverPath) {
    window.showErrorMessage(
      "Vize: Could not find language server. Please install vize or set vize.serverPath.",
    );
    return;
  }

  outputChannel.appendLine(`Using server: ${serverPath}`);

  // Configure the server
  const serverOptions: ServerOptions = createServerOptions(serverPath);

  // Configure the client
  const clientOptions: LanguageClientOptions = {
    documentSelector: [{ scheme: "file", language: "vue" }],
    synchronize: {
      fileEvents: workspace.createFileSystemWatcher("**/*.vue"),
    },
    outputChannel,
    traceOutputChannel: outputChannel,
    initializationOptions: {
      diagnostics: config.get<boolean>("diagnostics.enable", true),
      completion: config.get<boolean>("completion.enable", true),
      hover: config.get<boolean>("hover.enable", true),
      codeLens: config.get<boolean>("codeLens.enable", true),
      formatting: config.get<boolean>("formatting.enable", true),
    },
  };

  // Create the language client
  client = new LanguageClient("vize", "Vize Language Server", serverOptions, clientOptions);

  // Register commands
  context.subscriptions.push(
    commands.registerCommand("vize.restartServer", async () => {
      outputChannel.appendLine("Restarting language server...");
      if (client) {
        await client.stop();
        await client.start();
        outputChannel.appendLine("Language server restarted");
      }
    }),

    commands.registerCommand("vize.showOutput", () => {
      outputChannel.show();
    }),

    commands.registerCommand("vize.findReferences", async () => {
      const editor = window.activeTextEditor;
      if (editor) {
        await commands.executeCommand("editor.action.referenceSearch.trigger");
      }
    }),
  );

  // Start the client
  try {
    await client.start();
    outputChannel.appendLine("Vize language server started successfully");
  } catch (error) {
    outputChannel.appendLine(`Failed to start language server: ${String(error)}`);
    window.showErrorMessage(`Vize: Failed to start language server: ${String(error)}`);
  }

  context.subscriptions.push(client);
}

export async function deactivate(): Promise<void> {
  if (client) {
    await client.stop();
  }
}

/**
 * Find the path to the language server executable.
 */
async function findServerPath(
  context: ExtensionContext,
  config: ReturnType<typeof workspace.getConfiguration>,
): Promise<string | undefined> {
  const exeName = process.platform === "win32" ? "vize.exe" : "vize";

  // 1. Check user-configured path
  const configuredPath = config.get<string>("serverPath");
  if (configuredPath && fs.existsSync(configuredPath)) {
    outputChannel.appendLine(`Found server at configured path: ${configuredPath}`);
    return configuredPath;
  }

  // 2. Check cargo install location first (most common)
  const homeDir = process.env.HOME || process.env.USERPROFILE || "";
  const cargoPath = path.join(homeDir, ".cargo", "bin", exeName);
  if (fs.existsSync(cargoPath)) {
    outputChannel.appendLine(`Found server at cargo path: ${cargoPath}`);
    return cargoPath;
  }

  // 3. Check PATH
  const pathEnv = process.env.PATH || "";
  const pathSeparator = process.platform === "win32" ? ";" : ":";
  const pathDirs = pathEnv.split(pathSeparator);

  for (const dir of pathDirs) {
    const serverPath = path.join(dir, exeName);
    if (fs.existsSync(serverPath)) {
      outputChannel.appendLine(`Found server in PATH: ${serverPath}`);
      return serverPath;
    }
  }

  // 4. Check bundled server in extension
  const bundledPaths = [
    path.join(context.extensionPath, "dist", exeName),
    path.join(context.extensionPath, "server", exeName),
  ];

  for (const serverPath of bundledPaths) {
    if (fs.existsSync(serverPath)) {
      outputChannel.appendLine(`Found bundled server: ${serverPath}`);
      return serverPath;
    }
  }

  // 5. Development: check relative to vize project root
  const devPaths = [
    path.join(context.extensionPath, "..", "..", "target", "release", exeName),
    path.join(context.extensionPath, "..", "..", "target", "debug", exeName),
    // Also check workspace folders
    ...getWorkspaceDevPaths(exeName),
  ];

  for (const serverPath of devPaths) {
    if (fs.existsSync(serverPath)) {
      outputChannel.appendLine(`Found dev server: ${serverPath}`);
      return serverPath;
    }
  }

  outputChannel.appendLine("Server not found in any location");
  return undefined;
}

/**
 * Get development paths from workspace folders.
 */
function getWorkspaceDevPaths(exeName: string): string[] {
  const paths: string[] = [];
  const workspaceFolders = workspace.workspaceFolders;
  if (workspaceFolders) {
    for (const folder of workspaceFolders) {
      paths.push(path.join(folder.uri.fsPath, "target", "release", exeName));
      paths.push(path.join(folder.uri.fsPath, "target", "debug", exeName));
    }
  }
  return paths;
}

/**
 * Create server options for the language client.
 */
function createServerOptions(serverPath: string): ServerOptions {
  const run: Executable = {
    command: serverPath,
    args: ["lsp"],
    transport: TransportKind.stdio,
  };

  const debug: Executable = {
    command: serverPath,
    args: ["lsp", "--debug"],
    transport: TransportKind.stdio,
    options: {
      env: {
        ...process.env,
        RUST_BACKTRACE: "1",
      },
    },
  };

  return {
    run,
    debug,
  };
}
