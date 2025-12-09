<script lang="ts">
    import { createEventDispatcher } from 'svelte';
    
    export let userId: string;
    export let username: string;
    export let status: 'ready' | 'waiting' | 'error' = 'waiting';
    export let isOwner: boolean = false;
    export let isSelf: boolean = false;
    export let downloadProgress: number = 0; // 0-100, 0 means not downloading
    export let downloadSpeed: string = '';
    
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
    class:downloading={downloadProgress > 0 && downloadProgress < 100}
    role="button"
    tabindex={isSelf ? -1 : 0}
    oncontextmenu={handleContextMenu}
>
    <div class="user-info">
        <div class="user-header">
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
        
        {#if downloadProgress > 0 && downloadProgress < 100}
            <div class="download-status">
                <div class="progress-text">
                    <span class="percentage">{downloadProgress}%</span>
                    <span class="speed">{downloadSpeed}</span>
                </div>
                <div class="progress-bar-container">
                    <div class="progress-bar-fill" style="width: {downloadProgress}%"></div>
                </div>
            </div>
        {/if}
    </div>
</div>

<style>
    .user-item {
        display: flex;
        align-items: flex-start;
        gap: 0.5rem;
        padding: 0.625rem 0.5rem;
        border-radius: 4px;
        margin-bottom: 0.25rem;
        cursor: pointer;
        transition: background-color 0.2s;
    }

    .user-item:hover {
        background-color: var(--bg-tertiary);
    }

    .user-item.self {
        background-color: var(--bg-tertiary);
        cursor: default;
    }

    .user-item.downloading {
        background-color: rgba(33, 150, 243, 0.1);
    }

    .user-info {
        display: flex;
        flex-direction: column;
        gap: 0.375rem;
        flex: 1;
        min-width: 0;
    }

    .user-header {
        display: flex;
        align-items: center;
        gap: 0.5rem;
        min-width: 0;
    }

    .status-dot {
        width: 10px;
        height: 10px;
        border-radius: 50%;
        flex-shrink: 0;
        margin-top: 0.125rem;
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
        min-width: 0;
        overflow: hidden;
        text-overflow: ellipsis;
        white-space: nowrap;
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

    .download-status {
        display: flex;
        flex-direction: column;
        gap: 0.25rem;
        width: 100%;
        padding-left: 20px; /* Align with username */
    }

    .progress-text {
        display: flex;
        justify-content: space-between;
        align-items: center;
        font-size: 0.7rem;
        color: var(--text-secondary);
    }

    .percentage {
        font-weight: 600;
        color: var(--accent-blue);
    }

    .speed {
        color: var(--text-secondary);
        font-size: 0.65rem;
    }

    .progress-bar-container {
        width: 100%;
        height: 4px;
        background-color: var(--bg-secondary);
        border-radius: 2px;
        overflow: hidden;
    }

    .progress-bar-fill {
        height: 100%;
        background: linear-gradient(90deg, 
            var(--accent-blue) 0%, 
            #64b5f6 100%);
        transition: width 0.3s ease;
        border-radius: 2px;
    }
</style>