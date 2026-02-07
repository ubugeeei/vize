<script setup>
import { RadioGroup } from '../index'
</script>

<art title="RadioGroup" component="./RadioGroupRoot.vue" category="Forms" status="ready" tags="radio,group,form">
  <variant name="Default" default>
    <RadioGroup.Root>
      <RadioGroup.Item value="a">
        <RadioGroup.Indicator /> A
      </RadioGroup.Item>
      <RadioGroup.Item value="b">
        <RadioGroup.Indicator /> B
      </RadioGroup.Item>
      <RadioGroup.Item value="c">
        <RadioGroup.Indicator /> C
      </RadioGroup.Item>
    </RadioGroup.Root>
  </variant>

  <variant name="With Default Value">
    <RadioGroup.Root default-value="b">
      <RadioGroup.Item value="a">
        <RadioGroup.Indicator /> A
      </RadioGroup.Item>
      <RadioGroup.Item value="b">
        <RadioGroup.Indicator /> B
      </RadioGroup.Item>
      <RadioGroup.Item value="c">
        <RadioGroup.Indicator /> C
      </RadioGroup.Item>
    </RadioGroup.Root>
  </variant>

  <variant name="Disabled">
    <RadioGroup.Root disabled>
      <RadioGroup.Item value="a">
        <RadioGroup.Indicator /> A
      </RadioGroup.Item>
      <RadioGroup.Item value="b">
        <RadioGroup.Indicator /> B
      </RadioGroup.Item>
      <RadioGroup.Item value="c">
        <RadioGroup.Indicator /> C
      </RadioGroup.Item>
    </RadioGroup.Root>
  </variant>

  <variant name="Horizontal">
    <RadioGroup.Root orientation="horizontal">
      <RadioGroup.Item value="a">
        <RadioGroup.Indicator /> A
      </RadioGroup.Item>
      <RadioGroup.Item value="b">
        <RadioGroup.Indicator /> B
      </RadioGroup.Item>
      <RadioGroup.Item value="c">
        <RadioGroup.Indicator /> C
      </RadioGroup.Item>
    </RadioGroup.Root>
  </variant>
</art>
