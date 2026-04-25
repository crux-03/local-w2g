<script lang="ts">
    import type {
        WidgetEntry,
        WidgetState,
        WidgetStateUpload,
        WidgetStateDownload,
    } from "$lib/api/types";
    import UploadWidget from "./widgets/UploadWidget.svelte";
    import DownloadWidget from "./widgets/DownloadWidget.svelte";

    let { message }: { message: WidgetEntry } = $props();

    export function isUploadState(s: WidgetState): s is WidgetStateUpload {
        return s.kind === "upload";
    }
    export function isDownloadState(s: WidgetState): s is WidgetStateDownload {
        return s.kind === "download";
    }
</script>

{#if isUploadState(message.kind.state)}
    <UploadWidget {message} data={message.kind.state} />
{:else if isDownloadState(message.kind.state)}
    <DownloadWidget {message} data={message.kind.state} />
{/if}
