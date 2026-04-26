import { type UserReadinessView } from "$lib/api/types";
import { listen } from "@tauri-apps/api/event";
import { addError } from "./error.svelte";

var states = $state<UserReadinessView[]>([]);

listen<UserReadinessView>("readiness_updated", (event) => {
  var idx = states?.findIndex((v) => v.user_id === event.payload.user_id);
  if (idx !== -1) {
    states[idx] = event.payload;
  } else {
    states.push(event.payload);
  }
}).catch((error) => {
  addError(`Failed to register readiness_updated listener: ${error}`);
});

export const stateStore = {
  get states() {
    return states;
  },
};
