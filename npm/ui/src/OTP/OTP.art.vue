<script setup>
import { OTP } from '../index'
</script>

<art title="OTP" component="./OTPRoot.vue" category="Forms" status="ready" tags="otp,pin,code,verification">
  <variant name="Default" default>
    <OTP.Root :length="6">
      <OTP.Slot :index="0" />
      <OTP.Slot :index="1" />
      <OTP.Slot :index="2" />
      <OTP.Slot :index="3" />
      <OTP.Slot :index="4" />
      <OTP.Slot :index="5" />
    </OTP.Root>
  </variant>

  <variant name="With Separator">
    <OTP.Root :length="6">
      <OTP.Slot :index="0" />
      <OTP.Slot :index="1" />
      <OTP.Slot :index="2" />
      <OTP.Separator />
      <OTP.Slot :index="3" />
      <OTP.Slot :index="4" />
      <OTP.Slot :index="5" />
    </OTP.Root>
  </variant>

  <variant name="Disabled">
    <OTP.Root :length="6" disabled>
      <OTP.Slot :index="0" />
      <OTP.Slot :index="1" />
      <OTP.Slot :index="2" />
      <OTP.Slot :index="3" />
      <OTP.Slot :index="4" />
      <OTP.Slot :index="5" />
    </OTP.Root>
  </variant>
</art>
