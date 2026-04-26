<script lang="ts">
    import type { SystemEntry } from "$lib/api/types";
    import { userStore } from "$src/lib/stores/users.svelte";

    let {
        message,
    }: {
        message: SystemEntry;
    } = $props();

    let date = $derived(new Date(message.timestamp));
    const permissions = [
        "MANAGE_PLAYBACK",
        "MANAGE_MEDIA",
        "MANAGE_USERS",
        "SEND_STATE",
        "SEND_MESSAGE",
    ];
    const usernames = $derived(
        userStore.users.map((u) => u.display_name ?? u.id),
    );

    type SegmentType =
        | "text"
        | "username"
        | "permission"
        | "space"
        | "highlight"
        | "category";
    type Segment = { type: SegmentType; text: string };

    function escapeRegex(s: string) {
        return s.replace(/[.*+?^${}()|[\]\\]/g, "\\$&");
    }

    // Split a chunk into whitespace and non-whitespace segments. Whitespace
    // becomes its own "space" segment so we can give it a CSS rule that
    // prevents collapse, regardless of what the template parser does around it.
    function pushSplit(
        out: Segment[],
        text: string,
        type: "text" | "username" | "permission" | "highlight" | "category",
    ) {
        const re = /(\s+)/g;
        let i = 0;
        for (const m of text.matchAll(re)) {
            if (m.index! > i) out.push({ type, text: text.slice(i, m.index) });
            out.push({ type: "space", text: m[0] });
            i = m.index! + m[0].length;
        }
        if (i < text.length) out.push({ type, text: text.slice(i) });
    }

    let segments = $derived.by<Segment[]>(() => {
        const content = message.kind.content;
        const users = [...usernames]
            .sort((a, b) => b.length - a.length)
            .map(escapeRegex);
        const perms = [...permissions]
            .sort((a, b) => b.length - a.length)
            .map(escapeRegex);

        const parts: string[] = [
            `(?<highlight>\\|-(?<highlightInner>.+?)-\\|)`,
            `(?<category>\\[[A-Z][A-Z0-9_-]*\\])`,
        ];
        if (users.length) parts.push(`(?<user>${users.join("|")})`);
        if (perms.length) parts.push(`(?<perm>${perms.join("|")})`);

        const out: Segment[] = [];
        const re = new RegExp(parts.join("|"), "g");
        let i = 0;
        for (const m of content.matchAll(re)) {
            if (m.index! > i) pushSplit(out, content.slice(i, m.index), "text");
            if (m.groups?.highlight !== undefined) {
                pushSplit(out, m.groups.highlightInner!, "highlight");
            } else if (m.groups?.category) {
                pushSplit(out, m[0], "category");
            } else if (m.groups?.user) {
                pushSplit(out, m[0], "username");
            } else {
                pushSplit(out, m[0], "permission");
            }
            i = m.index! + m[0].length;
        }
        if (i < content.length) pushSplit(out, content.slice(i), "text");
        return out;
    });
</script>

<div class="container">
    <span class="marker">system</span>
    <!-- prettier-ignore -->
    <span class="content">{#each segments as seg}<span class={seg.type}>{seg.text}</span>{/each}</span>
    <span class="timestamp">{date.toLocaleString()}</span>
</div>

<style>
    .container {
        display: flex;
        flex-direction: row;
        align-items: baseline;
        gap: var(--space-2);
        padding: var(--space-2) var(--space-4);
        font-size: var(--text-xs);
        color: var(--color-text-muted);
        font-style: italic;
    }
    .marker {
        flex-shrink: 0;
        font-family: var(--font-mono);
        font-style: normal;
        text-transform: uppercase;
        letter-spacing: var(--tracking-wide);
        font-weight: var(--font-medium);
        color: var(--color-text-subtle);
    }
    .content {
        flex: 1;
        min-width: 0;
    }
    .timestamp {
        flex-shrink: 0;
        font-style: normal;
        color: var(--color-text-subtle);
        font-variant-numeric: tabular-nums;
    }
    .username {
        color: var(--twilight-300);
        font-style: normal;
        font-weight: var(--font-medium);
    }
    .permission {
        color: var(--parchment-300);
        font-style: normal;
        font-family: var(--font-mono);
        font-size: 0.95em;
    }
    .highlight {
        font-style: normal;
        font-weight: var(--font-medium);
        color: var(--blue-400);
    }
    .category {
        font-style: normal;
        font-family: var(--font-mono);
        font-size: 0.95em;
        color: var(--moss-300);
    }
    .text {
        margin-right: 2px;
    }
</style>
