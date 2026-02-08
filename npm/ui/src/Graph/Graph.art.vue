<script setup>
import { Graph } from '../index'
</script>

<art title="Graph" component="./GraphRoot.vue" category="Data Display" status="ready" tags="graph,chart,visualization,svg">
  <variant name="Line Chart" default>
    <Graph.Root
      :data="[{x:0,y:10},{x:1,y:40},{x:2,y:30},{x:3,y:60},{x:4,y:50}]"
      :width="400"
      :height="200"
      x-accessor="x"
      y-accessor="y"
    >
      <Graph.Grid />
      <Graph.Axis position="bottom" />
      <Graph.Axis position="left" />
      <Graph.Line />
    </Graph.Root>
  </variant>

  <variant name="Bar Chart">
    <Graph.Root
      :data="[{x:0,y:10},{x:1,y:40},{x:2,y:30},{x:3,y:60},{x:4,y:50}]"
      :width="400"
      :height="200"
      x-accessor="x"
      y-accessor="y"
    >
      <Graph.Grid />
      <Graph.Axis position="bottom" />
      <Graph.Axis position="left" />
      <Graph.Bar />
    </Graph.Root>
  </variant>

  <variant name="Area Chart">
    <Graph.Root
      :data="[{x:0,y:10},{x:1,y:40},{x:2,y:30},{x:3,y:60},{x:4,y:50}]"
      :width="400"
      :height="200"
      x-accessor="x"
      y-accessor="y"
    >
      <Graph.Grid />
      <Graph.Axis position="bottom" />
      <Graph.Axis position="left" />
      <Graph.Area />
      <Graph.Line />
    </Graph.Root>
  </variant>

  <variant name="With Dots">
    <Graph.Root
      :data="[{x:0,y:10},{x:1,y:40},{x:2,y:30},{x:3,y:60},{x:4,y:50}]"
      :width="400"
      :height="200"
      x-accessor="x"
      y-accessor="y"
    >
      <Graph.Grid />
      <Graph.Axis position="bottom" />
      <Graph.Axis position="left" />
      <Graph.Line />
      <Graph.Dot />
    </Graph.Root>
  </variant>
</art>
