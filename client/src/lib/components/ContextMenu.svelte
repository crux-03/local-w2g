<script lang="ts">
    import { createEventDispatcher } from "svelte";

    export let userId: string;
    export let username: string;
    export let x: number;
    export let y: number;
    export let allowPause: boolean = false;
    export let allowSeek: boolean = false;
    export let allowSubtitle: boolean = false;
    export let allowAudio: boolean = false;

    const dispatch = createEventDispatcher();

    function handleTransferOwnership() {
        dispatch("transferOwnership", { userId });
    }

    function handleTogglePause() {
        dispatch("togglePermission", {
            userId,
            permission: "pause",
            value: !allowPause,
        });
    }

    function handleToggleSeek() {
        dispatch("togglePermission", {
            userId,
            permission: "seek",
            value: !allowSeek,
        });
    }

    function handleToggleSubtitle() {
        dispatch("togglePermission", {
            userId,
            permission: "subtitle",
            value: !allowSubtitle,
        });
    }

    function handleToggleAudio() {
        dispatch("togglePermission", {
            userId,
            permission: "audio",
            value: !allowAudio,
        });
    }
</script>

<div
    class="context-menu"
    style="left: {x}px; top: {y}px;"
    role="menu"
    tabindex="0"
    onclick={(e) => e.stopPropagation()}
    onkeydown={(e) => e.stopPropagation()}
>
    <div class="context-menu-header">
        <span class="context-menu-title">{username}</span>
    </div>
    
    <button
        type="button"
        class="context-menu-item danger"
        role="menuitem"
        onclick={handleTransferOwnership}
    >
        👑 Transfer Ownership
    </button>
    
    <div class="context-menu-separator"></div>
    
    <div class="context-menu-section-title">Permissions</div>
    
    <button
        type="button"
        class="context-menu-item"
        role="menuitem"
        onclick={handleTogglePause}
    >
        <input type="checkbox" checked={allowPause} onclick={(e) => e.stopPropagation()} />
        Play/Pause
    </button>
    
    <button
        type="button"
        class="context-menu-item"
        role="menuitem"
        onclick={handleToggleSeek}
    >
        <input type="checkbox" checked={allowSeek} onclick={(e) => e.stopPropagation()} />
        Seek
    </button>
    
    <button
        type="button"
        class="context-menu-item"
        role="menuitem"
        onclick={handleToggleSubtitle}
    >
        <input type="checkbox" checked={allowSubtitle} onclick={(e) => e.stopPropagation()} />
        Subtitles
    </button>
    
    <button
        type="button"
        class="context-menu-item"
        role="menuitem"
        onclick={handleToggleAudio}
    >
        <input type="checkbox" checked={allowAudio} onclick={(e) => e.stopPropagation()} />
        Audio Tracks
    </button>
</div>

<style>
    .context-menu {
        position: fixed;
        background-color: var(--bg-secondary);
        border: 1px solid var(--border-color);
        border-radius: 6px;
        padding: 0.25rem;
        box-shadow: 0 4px 12px var(--shadow);
        z-index: 1000;
        min-width: 200px;
    }

    .context-menu-header {
        padding: 0.5rem 0.75rem;
        border-bottom: 1px solid var(--border-color);
        margin-bottom: 0.25rem;
    }

    .context-menu-title {
        font-size: 0.875rem;
        font-weight: 600;
        color: var(--text-primary);
    }

    .context-menu-section-title {
        padding: 0.5rem 0.75rem;
        font-size: 0.75rem;
        font-weight: 600;
        color: var(--text-secondary);
        text-transform: uppercase;
        letter-spacing: 0.5px;
    }

    .context-menu-item {
        width: 100%;
        text-align: left;
        padding: 0.5rem 0.75rem;
        font-size: 0.875rem;
        cursor: pointer;
        border-radius: 4px;
        display: flex;
        align-items: center;
        gap: 0.5rem;
        background: transparent;
        border: none;
        color: var(--text-primary);
    }

    .context-menu-item:hover {
        background-color: var(--bg-tertiary);
    }

    .context-menu-item.danger {
        color: var(--accent-red);
    }

    .context-menu-item.danger:hover {
        background-color: rgba(244, 67, 54, 0.1);
    }

    .context-menu-item input[type="checkbox"] {
        margin: 0;
        padding: 0;
        width: auto;
        cursor: pointer;
    }

    .context-menu-separator {
        height: 1px;
        background-color: var(--border-color);
        margin: 0.25rem 0;
    }

    @media (prefers-color-scheme: light) {
        .context-menu {
            box-shadow: 0 4px 12px rgba(0, 0, 0, 0.15);
        }

        .context-menu-item.danger:hover {
            background-color: rgba(244, 67, 54, 0.05);
        }
    }
</style>