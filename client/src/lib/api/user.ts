import { type Permissions } from "$lib/api/types";

export interface User {
  id: string;
  display_name?: string;
  permissions: Permissions;
}
