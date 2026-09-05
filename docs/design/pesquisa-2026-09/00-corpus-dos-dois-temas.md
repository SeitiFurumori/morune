# Corpus — o que os dois temas são hoje

Medido em 05/09/2026, direto de `crates/morune-app/themes/*/theme.toml`.
Sem opinião ainda: só os números, para o resto da pesquisa ter contra o que
comparar.

## Bruma — o que quer ser Liquid Glass

| campo | valor |
|---|---|
| `background` | `#2e2e36a8` (alfa 168/255 = **66%**) |
| `surface` | `#ffffff1f` (branco a **12%**) |
| `surface_raised` | `#ffffff2e` (branco a **18%**) |
| `sidebar_background` | `#2a2a32b3` (70%) |
| `player_background` | `#26262db8` (72%) |
| `accent` | `#f2f4f8` — **branco levemente frio** |
| `text` / `text_muted` | `#f7f7f8` / `#e4e4e6` — **diferença de 4%** |
| `border` | `#ffffffaa` (67%) |
| `shadow` | `#00000000` — **transparente**, e `shadow_strength = 0.0` |
| `gloss` | 0.55 |
| `window_opacity` | 0.96 |
| `acrylic` | true |
| `backdrop_blur` | **0.0** |
| `artwork_tint` | true, força 0.22 |
| raios | sm 10 / md 18 / lg 26 / artwork 14 |
| espaçamento | xs 4 / sm 8 / md 14 / lg 22 / xl 34 |
| movimento | 120 / 200 / 320 ms, `ease-in-out` |

## Aquário — o que quer ser Frutiger Aero

| campo | valor |
|---|---|
| `background` | `#cfeafcf0` — azul-céu claro a 94% |
| `surface` | `#b7e86ae6` — **verde-limão** a 90% |
| `surface_raised` | `#cbf089f2` — verde mais claro a 95% |
| `sidebar_background` | `#ffffffc4` (77%) |
| `player_background` | `#ffffffd6` (84%) |
| `accent` | `#2fc0ee` — ciano |
| `text` | `#0e2b33` — azul-petróleo escuro |
| `border` | `#0a3a4a9e` (62%) |
| `shadow` | `#0a3a4a2e`, força 0.35 |
| `gloss` | **0.8** — o mais alto dos temas |
| `backdrop_blur` | **0.0** |
| raios | sm 9 / md 16 / lg 24 / artwork 12 |
| movimento | 140 / 240 / **400** ms, `ease-in-out` |
| fundo | `fundo.png`, cover, opacidade 0.92, tint branco 8% |

## Duas medidas que valem para os dois

**`backdrop_blur = 0.0` nos dois temas.** O campo existe no contrato de tema e
não é lido por nenhuma linha de interface — é token morto, como `density` e
`max_content_width`. O `Frost` de `components.slint` diz isso explicitamente:
*"Isto e um veu, nao desfoque. O Slint nao aplica filtro no que esta atras de um
elemento -- nao existe `backdrop-filter` aqui."* Qualquer conclusão desta
pesquisa que dependa de desfoque real esbarra nisso.

**Nenhum dos dois usa `display_family`.** Aquário repete `Segoe UI` nos dois
campos; Bruma nem declara. Depois de 05/09 o campo passou a valer no título
grande (ver `CHANGELOG`), então os dois deixam esse eixo em branco.
