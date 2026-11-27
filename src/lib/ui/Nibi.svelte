<script lang="ts">
  import { textos } from '#lib/i18n/index.ts';
  import type { EstadoNibi } from '#lib/core/ipc/index.ts';
  let { estado, compacto = false }: { estado: EstadoNibi; compacto?: boolean } = $props();
</script>

<div
  aria-hidden={compacto}
  class="habitat"
  class:mini={compacto}
  class:exterior={estado.escenario !== 'bg_default_room'}
  class:noche={estado.escenario === 'bg_space' || estado.escenario === 'bg_starry_dojo'}
>
  <svg
    viewBox="0 0 260 240"
    role="img"
    aria-label={estado.nombre || textos.nav.nibi}
    class={`dragon ${estado.color}`}
  >
    <g class="escenario" aria-hidden="true">
      {#if estado.escenario === 'bg_mountain_dawn' || estado.escenario === 'bg_snow'}
        <circle class="sol" cx="195" cy="54" r="23" /><path
          class="lejano"
          d="M0 157 60 54 119 144 176 70 260 157V240H0Z"
        /><path class="cercano" d="m30 240 93-133 92 133Z" />
        {#if estado.escenario === 'bg_snow'}<path
            class="nieve"
            d="m44 81 16-27 19 33-15-8-9 7Zm64 47 15-21 20 29-19-9-7 8Z"
          />{/if}
      {:else if estado.escenario === 'bg_forest' || estado.escenario === 'bg_sakura'}
        <path class="lejano" d="M0 164q65-36 128-3t132-8v87H0Z" />
        {#each [22, 64, 210, 250] as x}<path
            class="tronco"
            d={`M${x} 195v-95`}
          />{#if estado.escenario === 'bg_sakura'}<circle
              class="flores"
              cx={x}
              cy="87"
              r="29"
            />{:else}<path class="cercano" d={`m${x - 25} 150 25-90 25 90Z`} />{/if}{/each}
      {:else if estado.escenario === 'bg_beach'}
        <circle class="sol" cx="195" cy="64" r="25" /><path
          class="lejano"
          d="M0 130q65-12 130 0t130 0v110H0Z"
        /><path class="cercano" d="M0 194q65-18 130 0t130 0v46H0Z" /><path
          class="ola"
          d="M0 156q65-12 130 0t130 0"
        />
      {:else if estado.escenario === 'bg_space' || estado.escenario === 'bg_starry_dojo'}
        {#each [[20, 28], [60, 64], [180, 26], [232, 54], [21, 113], [225, 141], [210, 106]] as [x, y]}<circle
            class="estrella"
            cx={x}
            cy={y}
            r="2"
          />{/each}<circle class="luna" cx="215" cy="43" r="19" />
      {:else if estado.escenario === 'bg_zen_garden'}
        <path class="lejano" d="M0 176q130-26 260 0v64H0Z" /><path
          class="ola"
          d="M0 185q80-20 140 0t120 0M0 200q80-20 140 0t120 0M0 215q80-20 140 0t120 0"
        /><ellipse class="cercano" cx="34" cy="178" rx="24" ry="14" />
      {/if}
    </g>
    {#if estado.aura}<circle class="aura" cx="130" cy="135" r="89" />{/if}
    <ellipse class="suelo" cx="130" cy="216" rx="85" ry="10" />
    <path
      class="ala"
      d="M85 145Q35 110 42 76Q65 97 90 106ZM172 141Q225 111 219 77Q195 98 170 108Z"
    />
    <path class="piel" d="M164 167Q215 204 223 171Q237 206 203 215L157 205Z" />
    <path class="cresta" d="m199 193 12-15 7 14-13 8" />
    <ellipse class="piel" cx="130" cy="170" rx="50" ry="47" />
    <ellipse class="vientre" cx="130" cy="177" rx="30" ry="31" />
    <path class="piel" d="M89 187Q64 195 78 211H109M167 187Q193 197 177 211H151" />
    <path class="cuerno" d="M88 80 80 39Q101 46 107 69M153 69Q167 43 184 41L177 83" />
    <path class="piel" d="M73 112Q63 66 119 65Q180 56 188 106Q197 154 134 155Q77 159 73 112Z" />
    <path class="cresta" d="m111 67 12-20 12 17 12-13 9 16" />
    {#if estado.animo === 'sleeping' || estado.animo === 'zen'}<path
        class="rasgo"
        d="M93 110q9 8 18 0M149 110q9 8 18 0"
      />{:else}<ellipse class="ojo" cx="103" cy="110" rx="5" ry="8" /><ellipse
        class="ojo"
        cx="158"
        cy="110"
        rx="5"
        ry="8"
      />{/if}
    <ellipse class="mejilla" cx="91" cy="125" rx="10" ry="5" /><ellipse
      class="mejilla"
      cx="170"
      cy="125"
      rx="10"
      ry="5"
    />
    <path class="rasgo" d="M120 132q12 11 24 0" />
    {#if estado.sombrero}
      <g class="sombrero"
        >{#if estado.sombrero === 'hat_crown'}<path d="m94 64-3-29 21 15 17-25 17 25 22-16-3 30Z" />
        {:else if estado.sombrero === 'hat_chef'}<path
            d="M104 66V48Q83 46 91 31Q99 19 113 27Q123 9 139 24Q161 12 174 30Q183 48 159 50V65Z"
          />
        {:else if estado.sombrero === 'hat_wizard' || estado.sombrero === 'hat_party'}<path
            d="m95 67 36-58 36 58Z"
          />
        {:else if estado.sombrero === 'hat_zen_halo'}<ellipse
            cx="130"
            cy="39"
            rx="38"
            ry="7"
            fill="none"
            stroke="currentColor"
            stroke-width="5"
          />
        {:else if estado.sombrero === 'hat_flowers'}{#each [96, 113, 130, 147, 164] as x}<circle
              cx={x}
              cy="61"
              r="9"
            />{/each}
        {:else if estado.sombrero === 'hat_headband'}<path d="M86 73q42-17 89-2v12q-43-15-88 2Z" />
        {:else}<path d="M94 68Q94 31 131 34Q165 35 167 68L183 74H92Z" />{/if}</g
      >
    {/if}
    {#if estado.accesorio === 'acc_glasses' || estado.accesorio === 'acc_sunglasses'}<g
        class:sol={estado.accesorio === 'acc_sunglasses'}
        class="gafas"
        ><rect x="88" y="99" width="32" height="22" rx="7" /><rect
          x="141"
          y="99"
          width="32"
          height="22"
          rx="7"
        /><path d="M120 108h21" /></g
      >
    {:else if estado.accesorio === 'acc_bowtie'}<path
        class="sombrero"
        d="m109 153 20 10-20 12Zm43 0-22 10 22 12Z"
      />
    {:else if estado.accesorio === 'acc_scarf'}<path
        class="sombrero"
        d="M95 145q36 15 72 0v15q-36 15-72 0Zm53 15h14v30h-14Z"
      />
    {:else if estado.accesorio === 'acc_bell'}<circle
        class="sombrero"
        cx="130"
        cy="163"
        r="9"
      />{/if}
  </svg>
</div>

<style>
  .habitat {
    display: grid;
    place-items: center;
    min-height: 20rem;
    border-radius: var(--sc-radio-xl);
    background: var(--m3c-surface-container-high);
    overflow: hidden;
  }
  .habitat.exterior {
    background: linear-gradient(
      165deg,
      var(--m3c-secondary-container),
      var(--m3c-surface-container-low)
    );
  }
  .habitat.noche {
    background: var(--m3c-surface-container-lowest);
  }
  .dragon {
    width: min(100%, 23rem);
    --piel: var(--m3c-primary);
    --oscuro: var(--m3c-primary-container);
  }
  .piel {
    fill: var(--piel);
    stroke: var(--m3c-on-surface);
    stroke-width: 1.4;
  }
  .ala,
  .cresta {
    fill: var(--oscuro);
    stroke: var(--m3c-on-surface);
    stroke-width: 1.4;
  }
  .vientre,
  .cuerno {
    fill: var(--m3c-secondary-container);
  }
  .ojo {
    fill: var(--m3c-on-surface);
  }
  .rasgo {
    fill: none;
    stroke: var(--m3c-on-surface);
    stroke-width: 2.5;
    stroke-linecap: round;
  }
  .mejilla {
    fill: var(--m3c-tertiary);
    opacity: 0.45;
  }
  .suelo {
    fill: var(--m3c-outline-variant);
  }
  .sombrero {
    fill: var(--m3c-tertiary);
    color: var(--m3c-tertiary);
    stroke: var(--m3c-on-surface);
    stroke-width: 1.4;
  }
  .gafas {
    fill: none;
    stroke: var(--m3c-on-surface);
    stroke-width: 3;
  }
  .gafas.sol rect {
    fill: var(--m3c-on-surface);
  }
  .color_ocean {
    --piel: var(--sc-nibi-oceano);
  }
  .color_lilac {
    --piel: var(--sc-nibi-lila);
  }
  .color_coral {
    --piel: var(--sc-nibi-coral);
  }
  .color_ember {
    --piel: var(--sc-nibi-brasa);
  }
  .color_midnight {
    --piel: var(--sc-nibi-noche);
  }
  .color_gold {
    --piel: var(--sc-nibi-oro);
  }
  .habitat.mini {
    min-height: 8rem;
    border-radius: var(--sc-radio-md);
  }
  .mini .dragon {
    width: 8rem;
  }
  .escenario .lejano {
    fill: var(--m3c-secondary-container);
    opacity: 0.6;
  }
  .escenario .cercano {
    fill: var(--m3c-tertiary-container);
    opacity: 0.7;
  }
  .escenario .sol,
  .escenario .flores {
    fill: var(--m3c-tertiary);
    opacity: 0.45;
  }
  .escenario .nieve,
  .escenario .luna,
  .escenario .estrella {
    fill: var(--m3c-on-surface);
    opacity: 0.45;
  }
  .escenario .tronco {
    stroke: var(--m3c-outline);
    stroke-width: 5;
  }
  .escenario .ola {
    fill: none;
    stroke: var(--m3c-on-secondary-container);
    stroke-width: 1;
    opacity: 0.25;
  }
  .aura {
    fill: var(--m3c-primary);
    opacity: 0.08;
  }
</style>
