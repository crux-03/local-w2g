<script lang="ts">
    import type { ChatEntry } from "$lib/api/types";
    import { userStore } from "$src/lib/stores/users.svelte";

    let { message }: { message: ChatEntry } = $props();
    let user = $derived(
        userStore.users.find((u) => u.id === message.kind.sender),
    );
    let date = $derived(new Date(message.timestamp));
</script>

<div class="container">
    <div class="stack">
        <div class="line">
            <h4 class="username">
                {user?.display_name ?? user?.id ?? "Loading..."}
            </h4>
            <span class="timestamp">{date.toLocaleString()}</span>
        </div>

        <div class="content">{message.kind.content}</div>
    </div>
</div>

<style>
    .container {
        display: flex;
        flex-direction: row;
        padding: var(--space-2) var(--space-4);
    }
    .stack {
        display: flex;
        flex-direction: column;
    }
    .line {
        display: flex;
        flex-direction: row;
        align-items: baseline;
        gap: var(--space-2);
    }
    .username {
        margin: 0;
        display: flex;
        color: var(--color-secondary);
        font-weight: bold;
    }
    .timestamp {
        display: flex;
        font-size: var(--text-xs);
        color: var(--color-text-muted);
    }
    .content {
        font-size: var(--font-sm);
    }
</style>
