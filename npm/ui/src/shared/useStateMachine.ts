import { ref, type Ref } from 'vue'

type Machine<S extends string, E extends string> = {
  [state in S]: { [event in E]?: S }
}

export function useStateMachine<S extends string, E extends string>(
  initialState: S,
  machine: Machine<S, E>,
): { state: Ref<S>; dispatch: (event: E) => void } {
  const state = ref(initialState) as Ref<S>

  function dispatch(event: E) {
    const nextState = machine[state.value]?.[event]
    if (nextState) {
      state.value = nextState
    }
  }

  return { state, dispatch }
}
