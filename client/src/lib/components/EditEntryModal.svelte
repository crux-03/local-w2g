<script lang="ts">
    import { editStore } from "$lib/stores/edit.svelte";
    import { invoke } from "@tauri-apps/api/core";
    import type { TrackInfo } from "../api/media";
    import { Check, X } from "@lucide/svelte";

    let displayNameDraft = $state("");
    let audioTrack = $state(-1);
    let subtitleTrack = $state(-1);

    // Resync local drafts whenever the modal opens with a new entry.
    $effect(() => {
        if (editStore.entry) {
            displayNameDraft = String(editStore.entry.display_name);
            audioTrack = editStore.entry.audio_track;
            subtitleTrack = editStore.entry.subtitle_track;
        }
    });

    const dn = new Intl.DisplayNames(["en"], { type: "language" });

    function languageName(code: string | null | undefined): string | null {
        if (!code) return null;
        try {
            return dn.of(code) ?? code;
        } catch {
            return code;
        }
    }

    function trackLabel(t: TrackInfo): string {
        const lang = languageName(t.language);
        const title = t.title?.replace(/_/g, " ").trim() || null;
        if (lang && title) return `${lang} — ${title}`;
        if (lang) return lang;
        if (title) return title;
        return `Track ${t.index + 1}`;
    }

    async function commitDisplayName() {
        if (!editStore.entry) return;
        if (displayNameDraft === editStore.entry.display_name) return;
        await invoke("update_entry_display_name", {
            id: editStore.entry.id,
            displayName: displayNameDraft,
        });
    }

    async function commitAudioTrack() {
        if (!editStore.entry) return;
        await invoke("update_entry_audio_track", {
            id: editStore.entry.id,
            audioTrack,
        });
    }

    async function commitSubtitleTrack() {
        if (!editStore.entry) return;
        await invoke("update_entry_subtitle_track", {
            id: editStore.entry.id,
            subtitleTrack,
        });
    }

    function handleKeydown(e: KeyboardEvent) {
        if (e.key === "Escape") editStore.close();
    }
</script>

<svelte:window onkeydown={handleKeydown} />

{#if editStore.isOpen && editStore.entry && editStore.probe}
    {@const entry = editStore.entry}
    {@const probe = editStore.probe}

    <div class="modal-root">
        <button
            class="backdrop"
            onclick={editStore.close}
            tabindex="-1"
            aria-label="Close dialog"
        ></button>

        <div
            class="modal"
            role="dialog"
            aria-modal="true"
            aria-labelledby="edit-title"
        >
            <header>
                <h3 id="edit-title">Edit video</h3>
                <button
                    class="btn btn-ghost util-btn"
                    onclick={editStore.close}
                    aria-label="Close"><X /></button
                >
            </header>

            <div class="field">
                <label for="edit-display-name">Display name</label>
                <div class="row">
                    <input
                        id="edit-display-name"
                        class="input"
                        bind:value={displayNameDraft}
                        onkeydown={(e) =>
                            e.key === "Enter" && commitDisplayName()}
                    />
                    <button
                        class="btn btn-secondary util-btn"
                        onclick={commitDisplayName}
                        disabled={displayNameDraft ===
                            String(entry.display_name)}
                        aria-label="Save name"><Check /></button
                    >
                </div>
            </div>

            <div class="field">
                <label for="edit-audio-track">Audio track</label>
                <select
                    id="edit-audio-track"
                    class="input"
                    bind:value={audioTrack}
                    onchange={commitAudioTrack}
                >
                    <option value={-1}>Use file default</option>
                    {#each probe.audio_tracks as t (t.index)}
                        <option value={t.index}>{trackLabel(t)}</option>
                    {/each}
                </select>
            </div>

            <div class="field">
                <label for="edit-subtitle-track">Subtitle track</label>
                <select
                    id="edit-subtitle-track"
                    class="input"
                    bind:value={subtitleTrack}
                    onchange={commitSubtitleTrack}
                >
                    <option value={-1}>Use file default</option>
                    {#each probe.subtitle_tracks as t (t.index)}
                        <option value={t.index}>{trackLabel(t)}</option>
                    {/each}
                </select>
            </div>
        </div>
    </div>
{/if}

<style>
    .modal-root {
        position: fixed;
        inset: 0;
        z-index: var(--z-modal);
        display: flex;
        align-items: center;
        justify-content: center;
        padding: var(--space-4);
    }

    .backdrop {
        position: absolute;
        inset: 0;
        margin: 0;
        padding: 0;
        border: 0;
        background: var(--color-overlay);
        cursor: default;
    }

    .modal {
        position: relative; /* sits above .backdrop in source order */
        width: 100%;
        max-width: 440px;
        background: var(--color-surface-raised);
        border: var(--border-thin) solid var(--color-border);
        border-radius: var(--radius-lg);
        box-shadow: var(--shadow-lg);
        padding: var(--space-4);
        display: flex;
        flex-direction: column;
        gap: var(--space-4);
    }

    header {
        display: flex;
        align-items: center;
        justify-content: space-between;
    }

    header h3 {
        margin: 0;
        font-family: var(--font-serif);
        font-size: var(--text-lg);
        color: var(--color-text);
    }

    .field {
        display: flex;
        flex-direction: column;
        gap: var(--space-1);
    }

    .field label {
        font-size: var(--text-sm);
        color: var(--color-text-muted);
    }

    .row {
        display: flex;
        gap: var(--space-2);
    }

    .row .input {
        flex: 1;
    }

    /* Custom select — matches .input but draws its own chevron */
    select.input {
        appearance: none;
        -webkit-appearance: none;
        -moz-appearance: none;
        cursor: pointer;
        padding-right: var(--space-8);
        background-color: var(--color-surface);
        background-image:
            linear-gradient(
                45deg,
                transparent 50%,
                var(--color-text-muted) 50%
            ),
            linear-gradient(
                135deg,
                var(--color-text-muted) 50%,
                transparent 50%
            );
        background-position:
            calc(100% - 18px) calc(50% - 1px),
            calc(100% - 13px) calc(50% - 1px);
        background-size: 5px 5px;
        background-repeat: no-repeat;
        transition:
            border-color var(--duration-fast) var(--ease-out),
            box-shadow var(--duration-fast) var(--ease-out);
    }

    select.input:hover {
        border-color: var(--color-border-strong);
    }

    select.input:focus {
        outline: none;
        border-color: var(--color-accent);
        box-shadow: var(--shadow-focus);
    }

    select.input:disabled {
        opacity: 0.6;
        cursor: not-allowed;
    }
</style>
