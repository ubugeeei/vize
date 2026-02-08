<script setup>
import { TreeViewRoot, TreeViewItem, TreeViewItemTrigger, TreeViewItemIndicator, TreeViewGroup } from './index'
</script>

<art title="TreeView" component="./TreeViewRoot.vue" category="Navigation" status="ready" tags="tree,treeview,hierarchy,nested">
  <variant name="Default" default>
    <TreeViewRoot>
      <TreeViewItem value="item-1">
        <TreeViewItemTrigger>
          <TreeViewItemIndicator />
          Parent 1
        </TreeViewItemTrigger>
        <TreeViewGroup>
          <TreeViewItem value="item-1-1">
            <TreeViewItemTrigger>Child 1-1</TreeViewItemTrigger>
          </TreeViewItem>
          <TreeViewItem value="item-1-2">
            <TreeViewItemTrigger>Child 1-2</TreeViewItemTrigger>
          </TreeViewItem>
        </TreeViewGroup>
      </TreeViewItem>
      <TreeViewItem value="item-2">
        <TreeViewItemTrigger>
          <TreeViewItemIndicator />
          Parent 2
        </TreeViewItemTrigger>
        <TreeViewGroup>
          <TreeViewItem value="item-2-1">
            <TreeViewItemTrigger>Child 2-1</TreeViewItemTrigger>
          </TreeViewItem>
          <TreeViewItem value="item-2-2">
            <TreeViewItemTrigger>Child 2-2</TreeViewItemTrigger>
          </TreeViewItem>
        </TreeViewGroup>
      </TreeViewItem>
    </TreeViewRoot>
  </variant>

  <variant name="Multiple Selection">
    <TreeViewRoot multiple>
      <TreeViewItem value="item-1">
        <TreeViewItemTrigger>
          <TreeViewItemIndicator />
          Parent 1
        </TreeViewItemTrigger>
        <TreeViewGroup>
          <TreeViewItem value="item-1-1">
            <TreeViewItemTrigger>Child 1-1</TreeViewItemTrigger>
          </TreeViewItem>
          <TreeViewItem value="item-1-2">
            <TreeViewItemTrigger>Child 1-2</TreeViewItemTrigger>
          </TreeViewItem>
        </TreeViewGroup>
      </TreeViewItem>
      <TreeViewItem value="item-2">
        <TreeViewItemTrigger>
          <TreeViewItemIndicator />
          Parent 2
        </TreeViewItemTrigger>
        <TreeViewGroup>
          <TreeViewItem value="item-2-1">
            <TreeViewItemTrigger>Child 2-1</TreeViewItemTrigger>
          </TreeViewItem>
          <TreeViewItem value="item-2-2">
            <TreeViewItemTrigger>Child 2-2</TreeViewItemTrigger>
          </TreeViewItem>
        </TreeViewGroup>
      </TreeViewItem>
    </TreeViewRoot>
  </variant>

  <variant name="Default Expanded">
    <TreeViewRoot :default-expanded="['item-1']">
      <TreeViewItem value="item-1">
        <TreeViewItemTrigger>
          <TreeViewItemIndicator />
          Parent 1
        </TreeViewItemTrigger>
        <TreeViewGroup>
          <TreeViewItem value="item-1-1">
            <TreeViewItemTrigger>Child 1-1</TreeViewItemTrigger>
          </TreeViewItem>
          <TreeViewItem value="item-1-2">
            <TreeViewItemTrigger>Child 1-2</TreeViewItemTrigger>
          </TreeViewItem>
        </TreeViewGroup>
      </TreeViewItem>
      <TreeViewItem value="item-2">
        <TreeViewItemTrigger>
          <TreeViewItemIndicator />
          Parent 2
        </TreeViewItemTrigger>
        <TreeViewGroup>
          <TreeViewItem value="item-2-1">
            <TreeViewItemTrigger>Child 2-1</TreeViewItemTrigger>
          </TreeViewItem>
          <TreeViewItem value="item-2-2">
            <TreeViewItemTrigger>Child 2-2</TreeViewItemTrigger>
          </TreeViewItem>
        </TreeViewGroup>
      </TreeViewItem>
    </TreeViewRoot>
  </variant>
</art>
