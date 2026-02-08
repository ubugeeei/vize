<script setup>
import { DatePickerRoot, DatePickerTrigger, DatePickerInput, DatePickerContent } from './index'
import { CalendarRoot, CalendarHeader, CalendarPrev, CalendarNext, CalendarHeading, CalendarGrid, CalendarGridHead, CalendarGridHeadCell, CalendarGridBody, CalendarCell, CalendarCellTrigger } from '../Calendar/index'
</script>

<art title="DatePicker" component="./DatePickerRoot.vue" category="Forms" status="ready" tags="datepicker,date,calendar,picker">
  <variant name="Default" skip-vrt>
    <DatePickerRoot v-slot="{ open, modelValue }">
      <DatePickerInput />
      <DatePickerTrigger>📅</DatePickerTrigger>
      <DatePickerContent disable-teleport>
        <CalendarRoot v-slot="{ months, weekDays }">
          <CalendarHeader>
            <CalendarPrev />
            <CalendarHeading />
            <CalendarNext />
          </CalendarHeader>
          <CalendarGrid v-for="month in months" :key="`${month.year}-${month.month}`">
            <CalendarGridHead>
              <tr>
                <CalendarGridHeadCell v-for="day in weekDays" :key="day">{{ day }}</CalendarGridHeadCell>
              </tr>
            </CalendarGridHead>
            <CalendarGridBody>
              <tr v-for="(week, i) in month.weeks" :key="i">
                <CalendarCell v-for="dayInfo in week" :key="dayInfo.date.toISOString()" :date="dayInfo.date">
                  <CalendarCellTrigger :date="dayInfo.date" />
                </CalendarCell>
              </tr>
            </CalendarGridBody>
          </CalendarGrid>
        </CalendarRoot>
      </DatePickerContent>
    </DatePickerRoot>
  </variant>

  <variant name="Default Open" skip-vrt>
    <DatePickerRoot v-slot="{ open, modelValue }">
      <DatePickerInput />
      <DatePickerTrigger>📅</DatePickerTrigger>
      <DatePickerContent disable-teleport force-mount>
        <CalendarRoot v-slot="{ months, weekDays }">
          <CalendarHeader>
            <CalendarPrev />
            <CalendarHeading />
            <CalendarNext />
          </CalendarHeader>
          <CalendarGrid v-for="month in months" :key="`${month.year}-${month.month}`">
            <CalendarGridHead>
              <tr>
                <CalendarGridHeadCell v-for="day in weekDays" :key="day">{{ day }}</CalendarGridHeadCell>
              </tr>
            </CalendarGridHead>
            <CalendarGridBody>
              <tr v-for="(week, i) in month.weeks" :key="i">
                <CalendarCell v-for="dayInfo in week" :key="dayInfo.date.toISOString()" :date="dayInfo.date">
                  <CalendarCellTrigger :date="dayInfo.date" />
                </CalendarCell>
              </tr>
            </CalendarGridBody>
          </CalendarGrid>
        </CalendarRoot>
      </DatePickerContent>
    </DatePickerRoot>
  </variant>
</art>
