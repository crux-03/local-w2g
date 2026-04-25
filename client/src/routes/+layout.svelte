<script lang="ts">
    import "$lib/assets/css/variables.css";
    import "$lib/assets/css/components.css";
    import { getCurrentWindow } from "@tauri-apps/api/window";

    const appWindow = getCurrentWindow();
</script>

<div class="shell">
    <header class="titlebar" data-tauri-drag-region>
        <span class="titlebar-title" data-tauri-drag-region>Watch2Gether</span>
        <div class="titlebar-controls">
            <button
                class="titlebar-btn"
                on:click={() => appWindow.minimize()}
                aria-label="Minimize"
            >
                &#x2014;
            </button>
            <button
                class="titlebar-btn"
                on:click={() => appWindow.toggleMaximize()}
                aria-label="Maximize"
            >
                &#x25A1;
            </button>
            <button
                class="titlebar-btn titlebar-btn-close"
                on:click={() => appWindow.close()}
                aria-label="Close"
            >
                &#x00D7;
            </button>
        </div>
    </header>

    <div class="content">
        <slot />
    </div>
</div>

<style>
    .shell {
        display: flex;
        flex-direction: column;
        height: 100vh;
    }

    .titlebar {
        display: flex;
        align-items: center;
        justify-content: space-between;
        height: var(--titlebar-height);
        padding-left: var(--space-3);
        background: var(--color-bg-subtle);
        border-bottom: var(--border-thin) solid var(--color-border);
        user-select: none;
        -webkit-user-select: none;
    }

    .titlebar-title {
        font-size: var(--text-sm);
        font-weight: var(--font-medium);
        color: var(--color-text-muted);
    }

    .titlebar-controls {
        display: flex;
        height: 100%;
    }

    .titlebar-btn {
        display: inline-flex;
        align-items: center;
        justify-content: center;
        width: 44px;
        height: 100%;
        padding: 0;
        font-size: var(--text-sm);
        color: var(--color-text-muted);
        background: transparent;
        border: 0;
        cursor: pointer;
        transition:
            background-color var(--duration-fast) var(--ease-out),
            color var(--duration-fast) var(--ease-out);
    }

    .titlebar-btn:hover {
        background: var(--color-border);
        color: var(--color-text);
    }

    .titlebar-btn-close:hover {
        background: var(--color-danger);
        color: #ffffff;
    }

    .content {
        flex: 1;
        min-height: 0;
        overflow: hidden;
    }
</style>
