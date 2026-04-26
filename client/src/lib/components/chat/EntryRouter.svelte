<script lang="ts">
    import type {
        ChatEntry,
        Entry,
        SystemEntry,
        WidgetEntry,
    } from "$lib/api/types";
    import ChatMessage from "./ChatMessage.svelte";
    import Widget from "./Widget.svelte";
    import SystemMessage from "./SystemMessage.svelte";

    let { message }: { message: Entry } = $props();

    export function isChatEntry(entry: Entry): entry is ChatEntry {
        return entry.kind.type === "chat";
    }
    export function isSystemEntry(entry: Entry): entry is SystemEntry {
        return entry.kind.type === "system";
    }
    export function isWidgetEntry(entry: Entry): entry is WidgetEntry {
        return entry.kind.type === "widget";
    }
</script>

{#if isChatEntry(message)}
    <ChatMessage {message} />
{:else if isSystemEntry(message)}
    <SystemMessage {message} />
{:else if isWidgetEntry(message)}
    <Widget {message} />
{/if}
