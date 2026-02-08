<script setup>
import { DatePicker, Calendar } from '../index'
</script>

<art title="DatePicker" component="./DatePickerRoot.vue" category="Forms" status="ready" tags="datepicker,date,calendar,picker">
  <variant name="Default" skip-vrt>
    <DatePicker.Root v-slot="{ open, modelValue }">
      <DatePicker.Input />
      <DatePicker.Trigger>📅</DatePicker.Trigger>
      <DatePicker.Content disable-teleport>
        <Calendar.Root v-slot="{ months, weekDays }">
          <Calendar.Header>
            <Calendar.Prev />
            <Calendar.Heading />
            <Calendar.Next />
          </Calendar.Header>
          <Calendar.Grid v-for="month in months" :key="`${month.year}-${month.month}`">
            <Calendar.GridHead>
              <tr>
                <Calendar.GridHeadCell v-for="day in weekDays" :key="day">{{ day }}</Calendar.GridHeadCell>
              </tr>
            </Calendar.GridHead>
            <Calendar.GridBody>
              <tr v-for="(week, i) in month.weeks" :key="i">
                <Calendar.Cell v-for="dayInfo in week" :key="dayInfo.date.toISOString()" :date="dayInfo.date">
                  <Calendar.CellTrigger :date="dayInfo.date" />
                </Calendar.Cell>
              </tr>
            </Calendar.GridBody>
          </Calendar.Grid>
        </Calendar.Root>
      </DatePicker.Content>
    </DatePicker.Root>
  </variant>

  <variant name="Default Open" skip-vrt>
    <DatePicker.Root v-slot="{ open, modelValue }">
      <DatePicker.Input />
      <DatePicker.Trigger>📅</DatePicker.Trigger>
      <DatePicker.Content disable-teleport force-mount>
        <Calendar.Root v-slot="{ months, weekDays }">
          <Calendar.Header>
            <Calendar.Prev />
            <Calendar.Heading />
            <Calendar.Next />
          </Calendar.Header>
          <Calendar.Grid v-for="month in months" :key="`${month.year}-${month.month}`">
            <Calendar.GridHead>
              <tr>
                <Calendar.GridHeadCell v-for="day in weekDays" :key="day">{{ day }}</Calendar.GridHeadCell>
              </tr>
            </Calendar.GridHead>
            <Calendar.GridBody>
              <tr v-for="(week, i) in month.weeks" :key="i">
                <Calendar.Cell v-for="dayInfo in week" :key="dayInfo.date.toISOString()" :date="dayInfo.date">
                  <Calendar.CellTrigger :date="dayInfo.date" />
                </Calendar.Cell>
              </tr>
            </Calendar.GridBody>
          </Calendar.Grid>
        </Calendar.Root>
      </DatePicker.Content>
    </DatePicker.Root>
  </variant>
</art>
