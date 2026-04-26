import type { Entry } from "./types";

export type MessageGroup = {
  user: string; // ID of the sender
  type: "chat" | "widget" | "system";
  entries: Entry[]; // All the grouped messages
};
