; Instalador do Morune.
;
; Objetivos, nesta ordem:
;   1. um unico .exe, sem pre-requisitos e sem runtime para instalar antes;
;   2. o usuario escolhe o disco e a pasta, com espaco livre visivel na hora;
;   3. sem UAC -- instalacao por usuario, como Discord e Spotify fazem.
;
; A instalacao por usuario e o que torna a escolha de disco realmente livre:
; com instalacao por maquina, escolher um disco secundario ainda exigiria
; elevacao, e recusar a pasta depois de o usuario escolher seria pior.
;
; Compilar com: tools\build-installer.ps1

Unicode true
ManifestDPIAware true

!include "MUI2.nsh"
!include "FileFunc.nsh"
!include "LogicLib.nsh"
!include "x64.nsh"

!ifndef VERSION
  !define VERSION "0.1.0"
!endif
!ifndef SOURCE_EXE
  !define SOURCE_EXE "..\target\release\morune.exe"
!endif

!define APP_NAME    "Morune"
!define APP_EXE     "morune.exe"
!define PUBLISHER   "Morune"
!define REG_UNINST  "Software\Microsoft\Windows\CurrentVersion\Uninstall\Morune"
!define REG_APP     "Software\Morune"

Name "${APP_NAME} ${VERSION}"
OutFile "..\dist\Morune-${VERSION}-setup.exe"
BrandingText "${APP_NAME} ${VERSION}"

; Sem elevacao: tudo que o instalador escreve fica no perfil do usuario ou na
; pasta que ele escolher.
RequestExecutionLevel user

; LZMA com dicionario solido: o executavel e o unico arquivo grande, e o ganho
; sobre ZLIB e da ordem de metade do tamanho final.
SetCompressor /SOLID lzma
SetCompressorDictSize 32

InstallDir "$LOCALAPPDATA\Programs\${APP_NAME}"
InstallDirRegKey HKCU "${REG_APP}" "InstallDir"

!define MUI_ABORTWARNING
; Icone da marca, o mesmo que fica no executavel. As entradas pequenas sao DIB
; porque o NSIS nao le entradas comprimidas em PNG nesses tamanhos.
!define MUI_ICON "..\assets\brand\morune.ico"
!define MUI_UNICON "..\assets\brand\morune.ico"

!define MUI_WELCOMEPAGE_TITLE "Instalar o ${APP_NAME}"
!define MUI_WELCOMEPAGE_TEXT "Cliente de musica nativo para Windows: leve, rapido e customizavel.$\r$\n$\r$\nNao e preciso instalar nada antes. Na proxima tela voce escolhe em qual disco e em qual pasta instalar."

; A pagina de diretorio ja mostra o espaco livre do disco selecionado, que e o
; que faz a escolha ser informada em vez de um chute.
!define MUI_DIRECTORYPAGE_TEXT_TOP "Escolha o disco e a pasta de instalacao. Clique em Procurar para trocar de disco.$\r$\n$\r$\nO ${APP_NAME} ocupa cerca de 10 MB. Suas configuracoes e temas ficam sempre em %APPDATA%, independente do disco escolhido aqui."
!define MUI_DIRECTORYPAGE_TEXT_DESTINATION "Pasta de instalacao"

!define MUI_FINISHPAGE_RUN "$INSTDIR\${APP_EXE}"
!define MUI_FINISHPAGE_RUN_TEXT "Abrir o ${APP_NAME} agora"
!define MUI_FINISHPAGE_LINK "Documentacao e codigo-fonte"
!define MUI_FINISHPAGE_LINK_LOCATION "https://github.com/morune/morune"

!insertmacro MUI_PAGE_WELCOME
!insertmacro MUI_PAGE_COMPONENTS
!insertmacro MUI_PAGE_DIRECTORY
!insertmacro MUI_PAGE_INSTFILES
!insertmacro MUI_PAGE_FINISH

!insertmacro MUI_UNPAGE_CONFIRM
!insertmacro MUI_UNPAGE_COMPONENTS
!insertmacro MUI_UNPAGE_INSTFILES

!insertmacro MUI_LANGUAGE "PortugueseBR"
!insertmacro MUI_LANGUAGE "English"

; Metadados de versao do instalador. O `LegalCopyright` acompanha a licenca
; escolhida em 18/08/2026 e o `OriginalFilename` deixa o arquivo identificavel
; mesmo depois de renomeado. Nenhum deles cala o SmartScreen -- so assinatura de
; codigo faz isso, e o porque esta em docs/assinatura.md.
VIProductVersion "${VERSION}.0"
VIAddVersionKey "ProductName" "${APP_NAME}"
VIAddVersionKey "FileDescription" "Instalador do ${APP_NAME}"
VIAddVersionKey "FileVersion" "${VERSION}"
VIAddVersionKey "ProductVersion" "${VERSION}"
VIAddVersionKey "CompanyName" "${PUBLISHER}"
VIAddVersionKey "LegalCopyright" "Copyright (c) 2026 Felipe Seiti Furumori. Licenca MIT."
VIAddVersionKey "OriginalFilename" "Morune-${VERSION}-setup.exe"

; ---------------------------------------------------------------------------

; Fecha o Morune se ele estiver rodando, inclusive escondido na bandeja.
;
; Deteccao sem plugin e sem busca em string: no formato CSV, `tasklist` responde
; `"morune.exe","1234",...` quando encontra o processo e `INFO: ...` quando nao
; encontra. Basta olhar o primeiro caractere.
Function CloseRunningApp
    nsExec::ExecToStack 'cmd /c tasklist /FI "IMAGENAME eq ${APP_EXE}" /NH /FO CSV'
    Pop $0   ; codigo de saida
    Pop $1   ; saida
    StrCpy $2 $1 1

    ${If} $2 == '"'
        MessageBox MB_OKCANCEL|MB_ICONEXCLAMATION \
            "O ${APP_NAME} esta aberto (pode estar apenas na bandeja, tocando).$\r$\n$\r$\nEle precisa ser fechado para continuar." \
            IDOK closeit
        Abort
        closeit:
        nsExec::ExecToLog 'taskkill /IM "${APP_EXE}" /F'
        Pop $0
        Sleep 1200
    ${EndIf}
FunctionEnd

; ---------------------------------------------------------------------------

Section "!${APP_NAME} (obrigatorio)" SecCore
    SectionIn RO
    SetOutPath "$INSTDIR"

    ; O binario e autocontido: nao ha DLL de runtime para copiar junto,
    ; verificado executando o .exe com um PATH minimo.
    File "${SOURCE_EXE}"
    File /nonfatal "..\README.md"
    ; Exigido pelas licencas das bibliotecas embutidas, nao e cortesia: MIT,
    ; Apache-2.0, BSD e ISC pedem que o aviso de copyright acompanhe o binario.
    File "..\LICENSE"
    File "..\THIRD-PARTY-LICENSES.txt"

    WriteRegStr HKCU "${REG_APP}" "InstallDir" "$INSTDIR"
    WriteRegStr HKCU "${REG_APP}" "Version" "${VERSION}"

    ; A secao opcional abaixo recria a entrada quando marcada. Remover aqui
    ; permite que desmarcar durante uma atualizacao realmente desligue a opcao.
    DeleteRegValue HKCU "Software\Microsoft\Windows\CurrentVersion\Run" "${APP_NAME}"

    WriteUninstaller "$INSTDIR\Uninstall.exe"

    ; Entrada em Aplicativos e Recursos.
    WriteRegStr   HKCU "${REG_UNINST}" "DisplayName"     "${APP_NAME}"
    WriteRegStr   HKCU "${REG_UNINST}" "DisplayVersion"  "${VERSION}"
    WriteRegStr   HKCU "${REG_UNINST}" "Publisher"       "${PUBLISHER}"
    WriteRegStr   HKCU "${REG_UNINST}" "DisplayIcon"     "$INSTDIR\${APP_EXE}"
    WriteRegStr   HKCU "${REG_UNINST}" "InstallLocation" "$INSTDIR"
    WriteRegStr   HKCU "${REG_UNINST}" "UninstallString" '"$INSTDIR\Uninstall.exe"'
    WriteRegStr   HKCU "${REG_UNINST}" "QuietUninstallString" '"$INSTDIR\Uninstall.exe" /S'
    WriteRegDWORD HKCU "${REG_UNINST}" "NoModify" 1
    WriteRegDWORD HKCU "${REG_UNINST}" "NoRepair" 1

    ${GetSize} "$INSTDIR" "/S=0K" $0 $1 $2
    IntFmt $0 "0x%08X" $0
    WriteRegDWORD HKCU "${REG_UNINST}" "EstimatedSize" "$0"
SectionEnd

Section "Atalho no menu Iniciar" SecStartMenu
    CreateDirectory "$SMPROGRAMS\${APP_NAME}"
    CreateShortcut "$SMPROGRAMS\${APP_NAME}\${APP_NAME}.lnk" "$INSTDIR\${APP_EXE}"
    CreateShortcut "$SMPROGRAMS\${APP_NAME}\Desinstalar ${APP_NAME}.lnk" "$INSTDIR\Uninstall.exe"
SectionEnd

Section "Atalho na area de trabalho" SecDesktop
    CreateShortcut "$DESKTOP\${APP_NAME}.lnk" "$INSTDIR\${APP_EXE}"
SectionEnd

Section /o "Abrir com o Windows" SecStartup
    ; Desmarcado por padrao: decidir sozinho que um player inicia junto com o
    ; sistema e o tipo de coisa que faz o usuario desinstalar.
    WriteRegStr HKCU "Software\Microsoft\Windows\CurrentVersion\Run" "${APP_NAME}" '"$INSTDIR\${APP_EXE}" --startup'
SectionEnd

Function .onInit
    ; Instalar por cima de uma versao em execucao deixaria um executavel
    ; travado e uma instalacao pela metade.
    Call CloseRunningApp

    ; Se ja existe instalacao, o padrao e atualizar no mesmo lugar em vez de
    ; espalhar copias por varios discos.
    ReadRegStr $0 HKCU "${REG_APP}" "InstallDir"
    ${If} $0 != ""
        StrCpy $INSTDIR $0
    ${EndIf}

    ; Reinstalar/atualizar reflete a escolha atual em vez de mostrar a caixa
    ; desmarcada quando o usuario ja habilitou pelo app ou por outro instalador.
    ReadRegStr $1 HKCU "Software\Microsoft\Windows\CurrentVersion\Run" "${APP_NAME}"
    ${If} $1 != ""
        SectionGetFlags ${SecStartup} $2
        IntOp $2 $2 | ${SF_SELECTED}
        SectionSetFlags ${SecStartup} $2
    ${EndIf}
FunctionEnd

!insertmacro MUI_FUNCTION_DESCRIPTION_BEGIN
    !insertmacro MUI_DESCRIPTION_TEXT ${SecCore}      "O aplicativo. Cerca de 10 MB, sem dependencias externas."
    !insertmacro MUI_DESCRIPTION_TEXT ${SecStartMenu} "Atalho no menu Iniciar."
    !insertmacro MUI_DESCRIPTION_TEXT ${SecDesktop}   "Atalho na area de trabalho."
    !insertmacro MUI_DESCRIPTION_TEXT ${SecStartup}   "Iniciar o ${APP_NAME} junto com o Windows."
!insertmacro MUI_FUNCTION_DESCRIPTION_END

; ---------------------------------------------------------------------------

Function un.onInit
    nsExec::ExecToLog 'taskkill /IM "${APP_EXE}" /F'
    Pop $0
    Sleep 800
FunctionEnd

Section "un.${APP_NAME}" UnCore
    SectionIn RO

    Delete "$INSTDIR\${APP_EXE}"
    Delete "$INSTDIR\README.md"
    Delete "$INSTDIR\LICENSE"
    Delete "$INSTDIR\THIRD-PARTY-LICENSES.txt"
    Delete "$INSTDIR\Uninstall.exe"
    ; `RMDir` sem /r: so remove se estiver vazio, para nunca apagar arquivos que
    ; o usuario tenha colocado na pasta.
    RMDir "$INSTDIR"

    Delete "$SMPROGRAMS\${APP_NAME}\${APP_NAME}.lnk"
    Delete "$SMPROGRAMS\${APP_NAME}\Desinstalar ${APP_NAME}.lnk"
    RMDir "$SMPROGRAMS\${APP_NAME}"
    Delete "$DESKTOP\${APP_NAME}.lnk"

    DeleteRegValue HKCU "Software\Microsoft\Windows\CurrentVersion\Run" "${APP_NAME}"
    DeleteRegKey HKCU "${REG_UNINST}"
    DeleteRegKey HKCU "${REG_APP}"
SectionEnd

Section /o "un.Configuracoes e temas" UnUserData
    ; Desmarcado por padrao: quem desinstala para reinstalar depois nao espera
    ; perder os temas que criou.
    RMDir /r "$APPDATA\morune"
    RMDir /r "$LOCALAPPDATA\morune"
SectionEnd

!insertmacro MUI_UNFUNCTION_DESCRIPTION_BEGIN
    !insertmacro MUI_DESCRIPTION_TEXT ${UnCore}     "Remove o aplicativo e os atalhos."
    !insertmacro MUI_DESCRIPTION_TEXT ${UnUserData} "Tambem apaga suas configuracoes e os temas que voce criou."
!insertmacro MUI_UNFUNCTION_DESCRIPTION_END
