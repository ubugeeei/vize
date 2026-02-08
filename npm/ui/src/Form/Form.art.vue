<script setup>
import { Form } from '../index'
</script>

<art title="Form" component="./FormRoot.vue" category="Forms" status="ready" tags="form,validation,schema">
  <variant name="Default" default>
    <Form.Root>
      <Form.Field name="email">
        <Form.Label>Email</Form.Label>
        <Form.Control placeholder="Enter your email" />
        <Form.Message />
      </Form.Field>
    </Form.Root>
  </variant>

  <variant name="With Submit">
    <Form.Root>
      <Form.Field name="email">
        <Form.Label>Email</Form.Label>
        <Form.Control placeholder="Enter your email" />
        <Form.Message />
      </Form.Field>
      <Form.Submit>Submit</Form.Submit>
    </Form.Root>
  </variant>

  <variant name="Multiple Fields">
    <Form.Root>
      <Form.Field name="email">
        <Form.Label>Email</Form.Label>
        <Form.Control placeholder="Enter your email" />
        <Form.Message />
      </Form.Field>
      <Form.Field name="password">
        <Form.Label>Password</Form.Label>
        <Form.Control as="input" type="password" placeholder="Enter your password" />
        <Form.Message />
      </Form.Field>
    </Form.Root>
  </variant>
</art>
