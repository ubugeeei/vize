<script setup>
import { CalendarRoot, CalendarHeader, CalendarPrev, CalendarNext, CalendarHeading, CalendarGrid, CalendarGridHead, CalendarGridHeadCell, CalendarGridBody, CalendarCell, CalendarCellTrigger } from './index'
</script>

<art title="Calendar" component="./CalendarRoot.vue" category="Date & Time" status="ready" tags="calendar,date,picker">
  <variant name="Default" default>
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
  </variant>

  <variant name="Multiple Selection">
    <CalendarRoot multiple v-slot="{ months, weekDays }">
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
  </variant>

  <variant name="Disabled">
    <CalendarRoot disabled v-slot="{ months, weekDays }">
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
  </variant>
</art>
