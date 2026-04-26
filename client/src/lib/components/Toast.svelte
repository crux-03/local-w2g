<script lang="ts">
    import { errorStore, dismissError } from "$lib/stores/error.svelte";
    import { fly } from "svelte/transition";
</script>

<div class="toast-container">
    {#each errorStore.errors as error (error.id)}
        <div
            class="toast"
            role="alert"
            transition:fly={{ x: 400, duration: 200 }}
        >
            <span>{error.message}</span>
            <button onclick={() => dismissError(error.id)} aria-label="Dismiss"
                >×</button
            >
        </div>
    {/each}
</div>

<style>
    .toast-container {
        position: fixed;
        top: var(--space-3);
        right: var(--space-3);
        display: flex;
        flex-direction: column;
        gap: var(--space-2);
        z-index: 1000;
        pointer-events: none;
    }
    .toast {
        pointer-events: auto;
        display: flex;
        align-items: center;
        gap: var(--space-2);
        padding: var(--space-2) var(--space-3);
        background: var(--color-surface);
        border: var(--border-thin) solid var(--color-danger, #e53935);
        border-left: 4px solid var(--color-danger, #e53935);
        border-radius: var(--radius-lg);
        color: var(--color-text);
        box-shadow: 0 4px 12px rgba(0, 0, 0, 0.15);
        min-width: 240px;
        max-width: 380px;
    }
    .toast button {
        background: none;
        border: none;
        color: inherit;
        cursor: pointer;
        font-size: 1.2rem;
        line-height: 1;
        margin-left: auto;
    }
</style>
