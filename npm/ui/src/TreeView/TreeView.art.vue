<script setup>
import { TreeView } from '../index'
</script>

<art title="TreeView" component="./TreeViewRoot.vue" category="Navigation" status="ready" tags="tree,treeview,hierarchy,nested">
  <variant name="Default" default>
    <TreeView.Root>
      <TreeView.Item value="item-1">
        <TreeView.ItemTrigger>
          <TreeView.ItemIndicator />
          Parent 1
        </TreeView.ItemTrigger>
        <TreeView.Group>
          <TreeView.Item value="item-1-1">
            <TreeView.ItemTrigger>Child 1-1</TreeView.ItemTrigger>
          </TreeView.Item>
          <TreeView.Item value="item-1-2">
            <TreeView.ItemTrigger>Child 1-2</TreeView.ItemTrigger>
          </TreeView.Item>
        </TreeView.Group>
      </TreeView.Item>
      <TreeView.Item value="item-2">
        <TreeView.ItemTrigger>
          <TreeView.ItemIndicator />
          Parent 2
        </TreeView.ItemTrigger>
        <TreeView.Group>
          <TreeView.Item value="item-2-1">
            <TreeView.ItemTrigger>Child 2-1</TreeView.ItemTrigger>
          </TreeView.Item>
          <TreeView.Item value="item-2-2">
            <TreeView.ItemTrigger>Child 2-2</TreeView.ItemTrigger>
          </TreeView.Item>
        </TreeView.Group>
      </TreeView.Item>
    </TreeView.Root>
  </variant>

  <variant name="Multiple Selection">
    <TreeView.Root multiple>
      <TreeView.Item value="item-1">
        <TreeView.ItemTrigger>
          <TreeView.ItemIndicator />
          Parent 1
        </TreeView.ItemTrigger>
        <TreeView.Group>
          <TreeView.Item value="item-1-1">
            <TreeView.ItemTrigger>Child 1-1</TreeView.ItemTrigger>
          </TreeView.Item>
          <TreeView.Item value="item-1-2">
            <TreeView.ItemTrigger>Child 1-2</TreeView.ItemTrigger>
          </TreeView.Item>
        </TreeView.Group>
      </TreeView.Item>
      <TreeView.Item value="item-2">
        <TreeView.ItemTrigger>
          <TreeView.ItemIndicator />
          Parent 2
        </TreeView.ItemTrigger>
        <TreeView.Group>
          <TreeView.Item value="item-2-1">
            <TreeView.ItemTrigger>Child 2-1</TreeView.ItemTrigger>
          </TreeView.Item>
          <TreeView.Item value="item-2-2">
            <TreeView.ItemTrigger>Child 2-2</TreeView.ItemTrigger>
          </TreeView.Item>
        </TreeView.Group>
      </TreeView.Item>
    </TreeView.Root>
  </variant>

  <variant name="Default Expanded">
    <TreeView.Root :default-expanded="['item-1']">
      <TreeView.Item value="item-1">
        <TreeView.ItemTrigger>
          <TreeView.ItemIndicator />
          Parent 1
        </TreeView.ItemTrigger>
        <TreeView.Group>
          <TreeView.Item value="item-1-1">
            <TreeView.ItemTrigger>Child 1-1</TreeView.ItemTrigger>
          </TreeView.Item>
          <TreeView.Item value="item-1-2">
            <TreeView.ItemTrigger>Child 1-2</TreeView.ItemTrigger>
          </TreeView.Item>
        </TreeView.Group>
      </TreeView.Item>
      <TreeView.Item value="item-2">
        <TreeView.ItemTrigger>
          <TreeView.ItemIndicator />
          Parent 2
        </TreeView.ItemTrigger>
        <TreeView.Group>
          <TreeView.Item value="item-2-1">
            <TreeView.ItemTrigger>Child 2-1</TreeView.ItemTrigger>
          </TreeView.Item>
          <TreeView.Item value="item-2-2">
            <TreeView.ItemTrigger>Child 2-2</TreeView.ItemTrigger>
          </TreeView.Item>
        </TreeView.Group>
      </TreeView.Item>
    </TreeView.Root>
  </variant>
</art>
