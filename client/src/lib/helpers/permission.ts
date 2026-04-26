import { type Permissions } from "$lib/api/types";

export function hasPermission(permissions: Permissions, flag: Permissions): boolean {
  return (permissions & flag) !== 0;
}

export function hasPermissions(permissions: Permissions, ...flags: Permissions[]): boolean {
  const mask = flags.reduce((acc, flag) => acc | flag, 0);
  return (permissions & mask) === mask;
}