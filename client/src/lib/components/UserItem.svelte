<script lang="ts">
    import { createEventDispatcher } from 'svelte';
    
    export let userId: string;
    export let username: string;
    export let status: 'ready' | 'waiting' | 'error' = 'waiting';
    export let isOwner: boolean = false;
    export let isSelf: boolean = false;
    
    const dispatch = createEventDispatcher();
    
    function handleContextMenu(event: MouseEvent) {
        if (!isSelf && isOwner) {
            // Only show context menu for other users if you're the owner
            dispatch('contextmenu', { userId, event });
        }
    }
</script>

<div 
    class="user-item" 
    class:self={isSelf}
    role="button"
    tabindex={isSelf ? -1 : 0}
    oncontextmenu={handleContextMenu}
>
    <span class="status-dot {status}"></span>
    <span class="username">
        {#if isSelf}
            {username} (me)
        {:else}
            {username}
        {/if}
    </span>
    {#if isOwner}
        <span class="owner-badge">Owner</span>
    {/if}
</div>

<style>
    .user-item {
        display: flex;
        align-items: center;
        gap: 0.5rem;
        padding: 0.5rem;
        border-radius: 4px;
        margin-bottom: 0.25rem;
        cursor: pointer;
    }

    .user-item:hover {
        background-color: var(--bg-tertiary);
    }

    .user-item.self {
        background-color: var(--bg-tertiary);
        cursor: default;
    }

    .status-dot {
        width: 10px;
        height: 10px;
        border-radius: 50%;
        flex-shrink: 0;
    }

    .status-dot.ready {
        background-color: var(--accent-green);
    }

    .status-dot.waiting {
        background-color: var(--accent-yellow);
    }

    .status-dot.error {
        background-color: var(--accent-red);
    }

    .username {
        flex: 1;
        font-size: 0.875rem;
    }

    .owner-badge {
        font-size: 0.625rem;
        padding: 0.125rem 0.375rem;
        background-color: var(--accent-blue);
        color: var(--text-primary);
        border-radius: 3px;
        text-transform: uppercase;
        font-weight: 600;
        flex-shrink: 0;
    }
</style>