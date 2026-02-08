<script setup>
import { RadioGroupRoot, RadioGroupItem, RadioGroupIndicator } from './index'
</script>

<art title="RadioGroup" component="./RadioGroupRoot.vue" category="Forms" status="ready" tags="radio,group,form">
  <variant name="Default" default>
    <RadioGroupRoot>
      <RadioGroupItem value="a">
        <RadioGroupIndicator /> A
      </RadioGroupItem>
      <RadioGroupItem value="b">
        <RadioGroupIndicator /> B
      </RadioGroupItem>
      <RadioGroupItem value="c">
        <RadioGroupIndicator /> C
      </RadioGroupItem>
    </RadioGroupRoot>
  </variant>

  <variant name="With Default Value">
    <RadioGroupRoot default-value="b">
      <RadioGroupItem value="a">
        <RadioGroupIndicator /> A
      </RadioGroupItem>
      <RadioGroupItem value="b">
        <RadioGroupIndicator /> B
      </RadioGroupItem>
      <RadioGroupItem value="c">
        <RadioGroupIndicator /> C
      </RadioGroupItem>
    </RadioGroupRoot>
  </variant>

  <variant name="Disabled">
    <RadioGroupRoot disabled>
      <RadioGroupItem value="a">
        <RadioGroupIndicator /> A
      </RadioGroupItem>
      <RadioGroupItem value="b">
        <RadioGroupIndicator /> B
      </RadioGroupItem>
      <RadioGroupItem value="c">
        <RadioGroupIndicator /> C
      </RadioGroupItem>
    </RadioGroupRoot>
  </variant>

  <variant name="Horizontal">
    <RadioGroupRoot orientation="horizontal">
      <RadioGroupItem value="a">
        <RadioGroupIndicator /> A
      </RadioGroupItem>
      <RadioGroupItem value="b">
        <RadioGroupIndicator /> B
      </RadioGroupItem>
      <RadioGroupItem value="c">
        <RadioGroupIndicator /> C
      </RadioGroupItem>
    </RadioGroupRoot>
  </variant>
</art>
