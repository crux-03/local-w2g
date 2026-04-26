<script lang="ts">
    import { Permissions } from "$lib/api/types";
    import { hasPermission } from "$lib/helpers/permission";
    import { userStore } from "$lib/stores/users.svelte";
    import { stateStore } from "$lib/stores/state.svelte";
    import { playlistStore } from "$lib/stores/playlist.svelte";
    import { invoke } from "@tauri-apps/api/core";
    import { Clipboard, Play, Check } from "@lucide/svelte";
    import { onMount } from "svelte";
    import { listen } from "@tauri-apps/api/event";
    import { formatPing } from "$lib/helpers/format";
    import { addError } from "../stores/error.svelte";

    const me = $derived(userStore.me);
    const selectedId = $derived(playlistStore.selected);
    const selectedEntry = $derived(
        selectedId
            ? playlistStore.entries.find((e) => e.id === selectedId)
            : undefined,
    );
    var ping = $derived("30ms");

    const totalViewers = $derived(stateStore.states.length);
    const readyCount = $derived(
        selectedId
            ? stateStore.states.filter(
                  (s) => s.videos[selectedId]?.status === "on_device",
              ).length
            : 0,
    );
    const readinessClass = $derived(
        totalViewers === 0
            ? ""
            : readyCount === totalViewers
              ? "ready"
              : readyCount === 0
                ? "not-ready"
                : "partial",
    );

    const hasManagePlaybackPerms = $derived(
        me ? hasPermission(me.permissions, Permissions.MANAGE_PLAYBACK) : false,
    );

    async function handlePlay() {
        try {
            await invoke("play", { videoId: selectedId });
        } catch (error) {
            addError(`Error when playing: ${error}`);
        }
    }

    let copied = $state(false);
    let copiedTimeout: ReturnType<typeof setTimeout> | undefined;

    async function handleCopyPassword() {
        try {
            // your existing copy logic here
            copied = true;
            clearTimeout(copiedTimeout);
            copiedTimeout = setTimeout(() => {
                copied = false;
            }, 1500);
        } catch (error) {
            addError(`Error when copying password: ${error}`);
        }
    }

    onMount(async () => {
        listen<number>("pong", (event) => {
            ping = formatPing(event.payload);
        }).catch((error) => {
            addError(`Failed to setup pong handler: ${error}`);
        });
    });
</script>

<div class="status">
    <div class="meta">
        <span class="ping">{ping}</span>
        <span class="dot" aria-hidden="true">·</span>
        <span>{totalViewers} viewer{totalViewers === 1 ? "" : "s"}</span>
        {#if hasManagePlaybackPerms}
            <button
                class="btn btn-ghost resync-btn"
                class:copied
                onclick={handleCopyPassword}
                title="Copy server password"
                aria-label="Copy server password"
            >
                {#if copied}
                    <Check />
                {:else}
                    <Clipboard />
                {/if}
            </button>
        {/if}
    </div>

    <div class="main">
        {#if selectedEntry}
            <span class="title" title={String(selectedEntry.display_name)}>
                {selectedEntry.display_name}
            </span>
            {#if totalViewers > 0}
                <span
                    class="readiness {readinessClass}"
                    aria-label="{readyCount} of {totalViewers} viewers ready"
                >
                    {readyCount}/{totalViewers} ready
                </span>
            {/if}
            {#if hasManagePlaybackPerms}
                <button class="btn btn-primary play-btn" onclick={handlePlay}>
                    <Play />
                    Start
                </button>
            {/if}
        {:else}
            <span class="empty">Pick a video from the playlist</span>
        {/if}
    </div>
</div>

<style>
    .status {
        display: flex;
        flex-direction: column;
        gap: var(--space-2);
        padding: var(--space-3) var(--space-4);
        background: var(--color-surface);
        border: var(--border-thin) solid var(--color-border);
        border-radius: var(--radius-md);
    }
    .meta {
        display: flex;
        align-items: center;
        gap: var(--space-2);
        font-size: var(--text-xs);
        color: var(--color-text-muted);
        text-transform: lowercase;
        letter-spacing: 0.05em;
    }
    .ping {
        font-variant-numeric: tabular-nums;
    }
    .dot {
        opacity: 0.5;
    }
    .resync-btn {
        margin-left: auto;
        padding: var(--space-1);
        max-width: 2em;
    }
    .resync-btn :global(svg) {
        width: 0.9rem;
        height: 0.9rem;
    }
    .resync-btn.copied {
        color: var(--color-success);
    }
    .main {
        display: flex;
        align-items: center;
        gap: var(--space-3);
        min-height: 1.75rem;
    }
    .title {
        flex: 1;
        min-width: 0;
        overflow: hidden;
        text-overflow: ellipsis;
        white-space: nowrap;
        font-weight: var(--font-medium);
        font-size: var(--text-sm);
    }
    .empty {
        flex: 1;
        color: var(--color-text-muted);
        font-style: italic;
        font-size: var(--text-sm);
    }
    .readiness {
        flex-shrink: 0;
        padding: var(--space-1) var(--space-2);
        border-radius: var(--radius-sm);
        font-size: var(--text-xs);
        font-variant-numeric: tabular-nums;
        border: var(--border-thin) solid var(--color-border);
    }
    .readiness.ready {
        color: var(--color-success);
        border-color: color-mix(in srgb, var(--color-success) 50%, transparent);
        background: color-mix(in srgb, var(--color-success) 12%, transparent);
    }
    .readiness.partial {
        color: var(--color-warning);
        border-color: color-mix(in srgb, var(--color-warning) 50%, transparent);
        background: color-mix(in srgb, var(--color-warning) 12%, transparent);
    }
    .readiness.not-ready {
        color: var(--color-danger);
        border-color: color-mix(in srgb, var(--color-danger) 50%, transparent);
        background: color-mix(in srgb, var(--color-danger) 12%, transparent);
    }
    .play-btn {
        flex-shrink: 0;
    }
    .play-btn :global(svg) {
        width: 0.9rem;
        height: 0.9rem;
        margin-right: var(--space-1);
    }
</style>
