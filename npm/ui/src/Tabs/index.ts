export { default as TabsRoot } from './TabsRoot.vue'
export { default as TabsList } from './TabsList.vue'
export { default as TabsTrigger } from './TabsTrigger.vue'
export { default as TabsContent } from './TabsContent.vue'
export { default as TabsIndicator } from './TabsIndicator.vue'
export type {
  TabsRootProps,
  TabsRootContext,
  ActivationMode,
  TabsListProps,
  TabsTriggerProps,
  TabsContentProps,
  TabsIndicatorProps,
} from './types'
export {
  injectTabsRootContext,
  provideTabsRootContext,
} from './types'
