<script setup>
import { OTPRoot, OTPSlot, OTPSeparator } from './index'
</script>

<art title="OTP" component="./OTPRoot.vue" category="Forms" status="ready" tags="otp,pin,code,verification">
  <variant name="Default" default>
    <OTPRoot :length="6">
      <OTPSlot :index="0" />
      <OTPSlot :index="1" />
      <OTPSlot :index="2" />
      <OTPSlot :index="3" />
      <OTPSlot :index="4" />
      <OTPSlot :index="5" />
    </OTPRoot>
  </variant>

  <variant name="With Separator">
    <OTPRoot :length="6">
      <OTPSlot :index="0" />
      <OTPSlot :index="1" />
      <OTPSlot :index="2" />
      <OTPSeparator />
      <OTPSlot :index="3" />
      <OTPSlot :index="4" />
      <OTPSlot :index="5" />
    </OTPRoot>
  </variant>

  <variant name="Disabled">
    <OTPRoot :length="6" disabled>
      <OTPSlot :index="0" />
      <OTPSlot :index="1" />
      <OTPSlot :index="2" />
      <OTPSlot :index="3" />
      <OTPSlot :index="4" />
      <OTPSlot :index="5" />
    </OTPRoot>
  </variant>
</art>
