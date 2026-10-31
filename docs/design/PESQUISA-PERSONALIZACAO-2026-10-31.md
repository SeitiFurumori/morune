# Como produtos reais fazem a personalização — pesquisa de 31/10/2026

Feita para reavaliar as decisões de agosto (ADR-0004 e contrato de 21/08), depois da entrevista de identidade ([ENTREVISTA-IDENTIDADE-2026-10-31.md](ENTREVISTA-IDENTIDADE-2026-10-31.md)).

## Casos

- **Wallpaper Engine.** Tem quatro tipos de wallpaper: vídeo, cena, web e aplicação. Só o tipo aplicação roda programa. A Kaspersky achou dezenas de wallpapers desse tipo com vírus no Workshop, com milhares de downloads. Em junho de 2026 a versão 2.8.42 tirou o tipo aplicação do Workshop, e os outros três continuam ([PCGuide](https://www.pcguide.com/news/wallpaper-engine-puts-an-end-to-exe-wallpapers-after-malware-spreads-on-the-steam-workshop/), [PC Gamer](https://pcgamer.com/hardware/warning-all-weebs-kaspersky-says-hackers-are-distributing-malware-via-anime-girl-wallpapers-on-steam-workshops-wallpaper-engine/)). Pausa ou para por aplicativo em tela cheia e tem limite de quadros.
- **Lively.** Pausa tudo com um aplicativo em tela cheia. Vídeo decodificado pela placa de vídeo custa quase o mesmo em qualquer qualidade ([wiki](https://github.com/rocksdanister/lively/wiki/Performance)).
- **Spicetify e Decky (Steam Deck).** Injetam código no app de outra empresa e quebram a cada atualização dela ([issue](https://github.com/spicetify/cli/issues/3485)).
- **Rainmeter, BetterDiscord, Obsidian.** Pacotes com código que roda sem proteção, e cada um tem histórico ou aviso de vírus.
- **Telegram Desktop.** Tema é um arquivo só com cores, ou um zip se tiver imagem de fundo. Tem editor dentro do app e prévia antes de aplicar, e se compartilha como arquivo comum ([wiki](https://github.com/telegramdesktop/tdesktop/wiki/Theme-Reference)).
- **Firefox e Discord.** Personalização por cor e imagem, sem código. O Firefox não aceita GIF animado.
- **Material You (Android).** Uma cor tirada do papel de parede gera a paleta inteira.
- **iCUE.** GIF na telinha do aparelho, gravado nele.
- **Plexamp.** Não tem skins. **Poweramp:** a skin é um app à parte, feito com um kit.

## Quatro modelos

| Modelo | Exemplos | Segurança | Para leigo | Poder do autor | Manutenção solo |
|---|---|---|---|---|---|
| A. Pronto + ajustes + mídia do usuário em pontos fixos | Discord, Firefox, Material You | alta | alta | baixo/médio | baixa |
| B. Arquivo de dados + editor no app que gera o arquivo | Telegram | alta | alta | médio | média |
| C. Camadas (fundo, peças, efeitos) com tipos de risco separados | Wallpaper Engine, Lively | alta sem o tipo com código | alta | alto | alta |
| D. Código livre (CSS/JS/scripts) | Spicetify, BetterDiscord, Rainmeter | baixa | baixa | máximo | quebra sempre |

## Não encontrado

- Nenhuma fonte sobre fazer o editor cedo ou tarde.
- Nenhuma medição de CPU, GPU ou memória de vídeo de fundo.
- Nada sobre Apple Music, VS Code, Razer, Maki ou foobar.
- O relatório da Kaspersky e o changelog oficial só foram lidos pela imprensa.
