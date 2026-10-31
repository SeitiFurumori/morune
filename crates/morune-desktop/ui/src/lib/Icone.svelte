<script lang="ts">
  import { icones, cheios } from './icones'
  let { nome, cheio = false, tamanho = 20 }: { nome: string; cheio?: boolean; tamanho?: number } = $props()
</script>

<svg class="ic" width={tamanho} height={tamanho} viewBox="0 0 24 24" aria-hidden="true"
  fill={cheio ? 'currentColor' : 'none'} stroke={cheio ? 'none' : 'currentColor'}
  stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round">
  <path d={cheio ? (cheios[nome] ?? icones[nome]) : icones[nome]} />
</svg>
