<script setup>
import { StepperRoot, StepperItem, StepperTrigger, StepperIndicator, StepperContent, StepperSeparator } from './index'
</script>

<art title="Stepper" component="./StepperRoot.vue" category="Navigation" status="ready" tags="stepper,wizard,steps,progress">
  <variant name="Default" default>
    <StepperRoot default-value="1">
      <StepperItem value="1">
        <StepperTrigger>
          <StepperIndicator>1</StepperIndicator>
          Step 1
        </StepperTrigger>
        <StepperContent>Content for step 1</StepperContent>
        <StepperSeparator />
      </StepperItem>
      <StepperItem value="2">
        <StepperTrigger>
          <StepperIndicator>2</StepperIndicator>
          Step 2
        </StepperTrigger>
        <StepperContent>Content for step 2</StepperContent>
        <StepperSeparator />
      </StepperItem>
      <StepperItem value="3">
        <StepperTrigger>
          <StepperIndicator>3</StepperIndicator>
          Step 3
        </StepperTrigger>
        <StepperContent>Content for step 3</StepperContent>
      </StepperItem>
    </StepperRoot>
  </variant>

  <variant name="Second Step Active">
    <StepperRoot default-value="2">
      <StepperItem value="1">
        <StepperTrigger>
          <StepperIndicator>1</StepperIndicator>
          Step 1
        </StepperTrigger>
        <StepperContent>Content for step 1</StepperContent>
        <StepperSeparator />
      </StepperItem>
      <StepperItem value="2">
        <StepperTrigger>
          <StepperIndicator>2</StepperIndicator>
          Step 2
        </StepperTrigger>
        <StepperContent>Content for step 2</StepperContent>
        <StepperSeparator />
      </StepperItem>
      <StepperItem value="3">
        <StepperTrigger>
          <StepperIndicator>3</StepperIndicator>
          Step 3
        </StepperTrigger>
        <StepperContent>Content for step 3</StepperContent>
      </StepperItem>
    </StepperRoot>
  </variant>

  <variant name="Linear">
    <StepperRoot default-value="1" linear>
      <StepperItem value="1">
        <StepperTrigger>
          <StepperIndicator>1</StepperIndicator>
          Step 1
        </StepperTrigger>
        <StepperContent>Content for step 1</StepperContent>
        <StepperSeparator />
      </StepperItem>
      <StepperItem value="2">
        <StepperTrigger>
          <StepperIndicator>2</StepperIndicator>
          Step 2
        </StepperTrigger>
        <StepperContent>Content for step 2</StepperContent>
        <StepperSeparator />
      </StepperItem>
      <StepperItem value="3">
        <StepperTrigger>
          <StepperIndicator>3</StepperIndicator>
          Step 3
        </StepperTrigger>
        <StepperContent>Content for step 3</StepperContent>
      </StepperItem>
    </StepperRoot>
  </variant>
</art>
