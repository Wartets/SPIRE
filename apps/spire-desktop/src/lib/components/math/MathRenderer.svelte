<script lang="ts">
  import LatexRenderer from "$lib/components/math/LatexRenderer.svelte";

  export let latex = "";
  export let mode: "rendered" | "raw" = "rendered";
  export let block = true;
</script>

<LatexRenderer {latex} {mode} {block} />
