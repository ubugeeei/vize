<script setup>
import { Calendar } from '../index'
</script>

<art title="Calendar" component="./CalendarRoot.vue" category="Date & Time" status="ready" tags="calendar,date,picker">
  <variant name="Default" default>
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
  </variant>

  <variant name="Multiple Selection">
    <Calendar.Root multiple v-slot="{ months, weekDays }">
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
  </variant>

  <variant name="Disabled">
    <Calendar.Root disabled v-slot="{ months, weekDays }">
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
  </variant>
</art>
