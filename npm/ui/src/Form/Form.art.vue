<script setup>
import { FormRoot, FormField, FormLabel, FormControl, FormMessage, FormSubmit } from './index'
</script>

<art title="Form" component="./FormRoot.vue" category="Forms" status="ready" tags="form,validation,schema">
  <variant name="Default" default>
    <FormRoot>
      <FormField name="email">
        <FormLabel>Email</FormLabel>
        <FormControl placeholder="Enter your email" />
        <FormMessage />
      </FormField>
    </FormRoot>
  </variant>

  <variant name="With Submit">
    <FormRoot>
      <FormField name="email">
        <FormLabel>Email</FormLabel>
        <FormControl placeholder="Enter your email" />
        <FormMessage />
      </FormField>
      <FormSubmit>Submit</FormSubmit>
    </FormRoot>
  </variant>

  <variant name="Multiple Fields">
    <FormRoot>
      <FormField name="email">
        <FormLabel>Email</FormLabel>
        <FormControl placeholder="Enter your email" />
        <FormMessage />
      </FormField>
      <FormField name="password">
        <FormLabel>Password</FormLabel>
        <FormControl as="input" type="password" placeholder="Enter your password" />
        <FormMessage />
      </FormField>
    </FormRoot>
  </variant>
</art>
