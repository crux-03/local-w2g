import type { User } from "$lib/api/user";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import type { Permissions, Snowflake } from "$lib/api/types";
import { compareSnowflake } from "$lib/helpers/compare";

let users = $state<User[]>([]);
let self = $state<Snowflake>();

// Set up once when the module loads
listen<User[]>("user_list", (event) => {
  users = event.payload.sort((a, b) => compareSnowflake(a.id, b.id));
}).catch((error) => {
  console.error(`Failed to register users listener: ${error}`);
});

listen<[Snowflake, Permissions]>("permission_update", (event) => {
  console.log(JSON.stringify(event.payload));
  const index = users.findIndex((u) => u.id === event.payload[0]);
  if (index !== -1) {
    users[index].permissions = event.payload[1];
  }
}).catch((error) => {
  console.error(`Failed to register permission_update listener: ${error}`);
});

export const userStore = {
  get users() {
    return users;
  },
  get me() {
    if (self) {
      return users.find((u) => u.id === self);
    }
  },
  async requestUsers() {
    try {
      await invoke("request_users");
    } catch (error) {
      console.log(`Error when requesting users: ${error}`);
    }
  },
  async identifySelf() {
    try {
      let id = (await invoke("get_user_id")) as Snowflake;
      console.log(id);
      self = id;
    } catch (error) {
      console.log(`Error when identifying self: ${error}`);
    }
  },
  async updatePermissions(
    target: Snowflake,
    perm: Permissions,
    granted: boolean,
  ) {
    try {
      await invoke("update_permissions", {
        targetUser: target,
        permission: perm,
        granted: granted,
      });
    } catch (error) {
      console.log(`Error when updating permissions: ${error}`);
    }
  },
};
