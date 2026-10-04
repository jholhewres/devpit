# Instalando o devpit

A versão curta é uma linha, no Linux ou no macOS: `curl -fsSL https://devpit.app/install.sh | sh`. O resto está abaixo.

## Requisitos

- **Linux**, X11 ou Wayland, é o build. macOS tem um build universal sem
  assinatura nem notarização. Windows tem um instalador sem assinatura de
  código, e o build de Linux também roda dentro do WSL — veja
  [Windows](#windows).
- **`tmux`**. Os terminais são painéis do tmux, e é por isso que eles
  sobrevivem à janela. No Windows é o psmux, e o instalador já o traz.
- **[Claude Code](https://claude.com/claude-code)** (`claude` no PATH). Chat,
  etapas de agente e de sessão passam por ele; é a única CLI que o devpit sabe
  conduzir hoje.

## Instalando

Linux e macOS estão na [última release][releases]. No Linux, prefira o
AppImage, a não ser que você tenha motivo para não: é o que se atualiza
sozinho.

Os comandos abaixo dizem `0.1.4` porque a versão faz parte do nome do arquivo.
Confira na página de releases qual é a atual — ou deixe um devpit instalado se
atualizar sozinho e nunca mais digite uma versão.

**AppImage (Linux).** O devpit procura uma versão nova, confere a assinatura,
instala por cima de si mesmo e reabre. Seus terminais continuam rodando no meio
disso — são sessões do tmux, e o tmux não cai junto com a janela.

```sh
curl -LO https://github.com/jholhewres/devpit/releases/latest/download/devpit_0.1.4_amd64.AppImage
chmod +x devpit_0.1.4_amd64.AppImage
./devpit_0.1.4_amd64.AppImage
```

**`.deb` (Debian, Ubuntu).** O devpit baixa a versão nova e confere, e então
mostra o comando sem nunca executá-lo. Instalar um pacote do sistema é pedir
root, e o devpit não pede root no seu lugar.

```sh
curl -LO https://github.com/jholhewres/devpit/releases/latest/download/devpit_0.1.4_amd64.deb
sudo apt install ./devpit_0.1.4_amd64.deb
```

**`.dmg` (macOS).** Só Apple silicon — M1 ou mais novo. Ainda não há build
para Intel: o runner que o produz é a última imagem Intel do GitHub e está
saindo de circulação, então a resposta para um Mac Intel é um binário
universal, que ainda não é construído. O devpit se atualiza aqui: baixa o
`.app`, confere a assinatura e se troca.

O download **não é assinado com um Developer ID da Apple nem notarizado**, então
a primeira abertura é recusada pelo Gatekeeper com "devpit está danificado" ou
"não pode ser aberto". Isso é o certificado que falta falando, não o arquivo.
Abra uma vez com botão direito → Abrir, ou tire a quarentena você mesmo:

```sh
xattr -dr com.apple.quarantine /Applications/devpit.app
```

Se for fazer isso, confira antes o `SHA256SUMS` — logo abaixo.

### Windows

Baixe o `devpit_<versão>_x64-setup.exe` da [última release][releases] e
execute. É um instalador NSIS que instala só para o seu usuário — sem direitos
de administrador — com o psmux, o tmux em que os terminais do devpit rodam lá,
dentro dele. Ele põe o devpit no menu Iniciar e sai limpo por Configurações →
Aplicativos. Precisa do Claude Code no PATH (`claude`) e do WebView2, que o
instalador baixa se o Windows não tiver.

O instalador **não é assinado com um certificado de assinatura de código**,
então o SmartScreen o barra na primeira vez com "O Windows protegeu o
computador". Isso é o certificado que falta falando, não o arquivo: confira
antes com o `SHA256SUMS` — no PowerShell,
`(Get-FileHash .\devpit_<versão>_x64-setup.exe).Hash.ToLower()` é a primeira
metade da linha — e então Mais informações → Executar assim mesmo.

Depois de instalado, o devpit se atualiza sozinho: baixa o instalador novo,
confere a assinatura, executa e reabre. Diferente do Linux e do macOS, **uma
atualização fecha seus terminais**: o Windows não troca um programa em
execução, então o instalador encerra antes o psmux de dentro do devpit. O card
avisa antes de reiniciar.

**Ou dentro do WSL.** O build de Linux roda no WSL 2, com a janela desenhada
pelo WSLg na área de trabalho do Windows. Os terminais são sessões tmux dentro
do WSL e sobrevivem à janela lá, atualizações incluídas.

1. Windows 11, ou Windows 10 21H2 em diante, com WSL 2 e uma distribuição
   Linux: `wsl --install` no PowerShell, depois reinicie.
2. No shell da distribuição: `sudo apt install tmux wslu`, e o Claude Code
   como diz a página de instalação dele.
3. A mesma linha do Linux, dentro do WSL:
   `curl -fsSL https://devpit.app/install.sh | sh`
4. Mantenha os projetos no sistema de arquivos do Linux (`~/…`), não em
   `/mnt/c`: git e os observadores de arquivo ficam muitas vezes mais lentos
   nessa fronteira.

O `wslu` é o que abre um link no navegador do Windows. Duas coisas não
atravessam: sessões logadas trazidas de um navegador do Windows, e arquivos
arrastados do Explorer.

Toda release traz um `SHA256SUMS`, e conferir é uma linha:

```sh
curl -LO https://github.com/jholhewres/devpit/releases/latest/download/SHA256SUMS
sha256sum -c SHA256SUMS --ignore-missing
```

O `.sig` ao lado de cada pacote é do updater, e não substitui isso: ele é o que
um devpit já instalado confere antes de se trocar, contra uma chave pública
compilada no binário que você já está rodando. Um primeiro download não tem
esse binário para conferir com ele — é para isso que serve o `SHA256SUMS`.

A checagem automática é um botão em Configurações → General, e vem ligada até
você desligar.

[releases]: https://github.com/jholhewres/devpit/releases/latest
