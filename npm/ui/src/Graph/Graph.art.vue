<script setup>
import { GraphRoot, GraphLine, GraphBar, GraphArea, GraphAxis, GraphGrid, GraphDot } from './index'
</script>

<art title="Graph" component="./GraphRoot.vue" category="Data Display" status="ready" tags="graph,chart,visualization,svg">
  <variant name="Line Chart" default>
    <GraphRoot
      :data="[{x:0,y:10},{x:1,y:40},{x:2,y:30},{x:3,y:60},{x:4,y:50}]"
      :width="400"
      :height="200"
      x-accessor="x"
      y-accessor="y"
    >
      <GraphGrid />
      <GraphAxis position="bottom" />
      <GraphAxis position="left" />
      <GraphLine />
    </GraphRoot>
  </variant>

  <variant name="Bar Chart">
    <GraphRoot
      :data="[{x:0,y:10},{x:1,y:40},{x:2,y:30},{x:3,y:60},{x:4,y:50}]"
      :width="400"
      :height="200"
      x-accessor="x"
      y-accessor="y"
    >
      <GraphGrid />
      <GraphAxis position="bottom" />
      <GraphAxis position="left" />
      <GraphBar />
    </GraphRoot>
  </variant>

  <variant name="Area Chart">
    <GraphRoot
      :data="[{x:0,y:10},{x:1,y:40},{x:2,y:30},{x:3,y:60},{x:4,y:50}]"
      :width="400"
      :height="200"
      x-accessor="x"
      y-accessor="y"
    >
      <GraphGrid />
      <GraphAxis position="bottom" />
      <GraphAxis position="left" />
      <GraphArea />
      <GraphLine />
    </GraphRoot>
  </variant>

  <variant name="With Dots">
    <GraphRoot
      :data="[{x:0,y:10},{x:1,y:40},{x:2,y:30},{x:3,y:60},{x:4,y:50}]"
      :width="400"
      :height="200"
      x-accessor="x"
      y-accessor="y"
    >
      <GraphGrid />
      <GraphAxis position="bottom" />
      <GraphAxis position="left" />
      <GraphLine />
      <GraphDot />
    </GraphRoot>
  </variant>
</art>
