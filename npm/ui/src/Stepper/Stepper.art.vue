<script setup>
import { Stepper } from '../index'
</script>

<art title="Stepper" component="./StepperRoot.vue" category="Navigation" status="ready" tags="stepper,wizard,steps,progress">
  <variant name="Default" default>
    <Stepper.Root default-value="1">
      <Stepper.Item value="1">
        <Stepper.Trigger>
          <Stepper.Indicator>1</Stepper.Indicator>
          Step 1
        </Stepper.Trigger>
        <Stepper.Content>Content for step 1</Stepper.Content>
        <Stepper.Separator />
      </Stepper.Item>
      <Stepper.Item value="2">
        <Stepper.Trigger>
          <Stepper.Indicator>2</Stepper.Indicator>
          Step 2
        </Stepper.Trigger>
        <Stepper.Content>Content for step 2</Stepper.Content>
        <Stepper.Separator />
      </Stepper.Item>
      <Stepper.Item value="3">
        <Stepper.Trigger>
          <Stepper.Indicator>3</Stepper.Indicator>
          Step 3
        </Stepper.Trigger>
        <Stepper.Content>Content for step 3</Stepper.Content>
      </Stepper.Item>
    </Stepper.Root>
  </variant>

  <variant name="Second Step Active">
    <Stepper.Root default-value="2">
      <Stepper.Item value="1">
        <Stepper.Trigger>
          <Stepper.Indicator>1</Stepper.Indicator>
          Step 1
        </Stepper.Trigger>
        <Stepper.Content>Content for step 1</Stepper.Content>
        <Stepper.Separator />
      </Stepper.Item>
      <Stepper.Item value="2">
        <Stepper.Trigger>
          <Stepper.Indicator>2</Stepper.Indicator>
          Step 2
        </Stepper.Trigger>
        <Stepper.Content>Content for step 2</Stepper.Content>
        <Stepper.Separator />
      </Stepper.Item>
      <Stepper.Item value="3">
        <Stepper.Trigger>
          <Stepper.Indicator>3</Stepper.Indicator>
          Step 3
        </Stepper.Trigger>
        <Stepper.Content>Content for step 3</Stepper.Content>
      </Stepper.Item>
    </Stepper.Root>
  </variant>

  <variant name="Linear">
    <Stepper.Root default-value="1" linear>
      <Stepper.Item value="1">
        <Stepper.Trigger>
          <Stepper.Indicator>1</Stepper.Indicator>
          Step 1
        </Stepper.Trigger>
        <Stepper.Content>Content for step 1</Stepper.Content>
        <Stepper.Separator />
      </Stepper.Item>
      <Stepper.Item value="2">
        <Stepper.Trigger>
          <Stepper.Indicator>2</Stepper.Indicator>
          Step 2
        </Stepper.Trigger>
        <Stepper.Content>Content for step 2</Stepper.Content>
        <Stepper.Separator />
      </Stepper.Item>
      <Stepper.Item value="3">
        <Stepper.Trigger>
          <Stepper.Indicator>3</Stepper.Indicator>
          Step 3
        </Stepper.Trigger>
        <Stepper.Content>Content for step 3</Stepper.Content>
      </Stepper.Item>
    </Stepper.Root>
  </variant>
</art>
