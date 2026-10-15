# Sam

Um app de macOS que encontra o que as ferramentas de desenvolvimento acumulam no seu Mac (caches do npm e do Homebrew, `node_modules` de projetos parados, builds antigos, imagens do Docker, sobras de apps removidos) e explica cada item antes de você decidir o que limpar.

O nome vem do Sam de *O Senhor dos Anéis*, o jardineiro que arruma o Condado no fim da história.

## Instalar

Baixe o `.dmg` da versão mais recente em **[Releases](https://github.com/Pedro-B-Siqueira/sam/releases/latest)**, abra e arraste o Sam para Aplicativos. Funciona no macOS 14 ou superior, em Macs Apple Silicon e Intel.

O app não tem assinatura da Apple, então na primeira vez o macOS bloqueia. Libere em **Ajustes → Privacidade e Segurança → Abrir mesmo assim**, ou rode:

```bash
xattr -dr com.apple.quarantine /Applications/Sam.app
```

## Como usar

1. Abra o Sam. Ele mede tudo sozinho; nada é apagado nesse momento.
2. O canteiro no topo mostra onde está o espaço: cada coluna é uma categoria, e as áreas cheias são as que estão marcadas para limpar.
3. Em cada categoria, **Entender antes de limpar** explica o que é, por que cresce, o que acontece se limpar, se dá para desfazer e o que o Sam recomenda.
4. Marque ou desmarque o que quiser e clique em **Revisar e limpar**. A confirmação lista tudo de novo, com o efeito de cada grupo, antes de apagar qualquer coisa.
5. No fim, o Sam mostra quanto espaço voltou e deixa salvar um relatório em Markdown.

## O que ele limpa

| Categoria | Vem marcada? | Dá para desfazer? |
|---|---|---|
| Cache do npm | sim | recriado sob demanda |
| Homebrew (instaladores e versões antigas) | sim | recriado sob demanda |
| Caches de aplicativos (`~/Library/Caches`) | sim | recriado sob demanda |
| Caches de ferramentas (Cypress, Playwright, Gradle, cargo, Yarn, pnpm, versões antigas do Claude Code…) | sim | recriado sob demanda |
| Saídas de build dos projetos (`.angular/cache`, `dist`, `build`…) | sim | recriado sob demanda |
| Builds do Rust (`target/`) | sim | recriado sob demanda |
| Docker: imagens sem uso e cache de build | sim | recriado sob demanda |
| `node_modules` de projetos parados | não | recriado com `npm ci` |
| Versões antigas do Node (nvm) | não | recriado com `nvm install` |
| Docker: volumes sem uso e contêineres parados | não | **não tem volta** |
| Sobras do Docker Desktop removido | não | **não tem volta** |
| Sobras de apps removidos | não | vai para a Lixeira |
| Downloads grandes e antigos | não | vai para a Lixeira |

Categorias que não se aplicam ao seu Mac (sem Homebrew, sem Docker, sem nvm…) aparecem explicando por quê, sem nada para limpar.

## Regras de segurança

- **Medir nunca apaga.** Só o botão de limpar, depois da tela de confirmação, apaga.
- **Só o que o Sam encontrou.** A limpeza aceita apenas itens da última medição, dentro de pastas conhecidas, e nunca segue atalhos (symlinks) para fora delas.
- **Nada versionado.** Dentro dos projetos, só entram pastas que o git ignora. Código e arquivos versionados nunca aparecem.
- **Projetos em uso ficam de fora.** `node_modules` só é oferecido em projetos sem commit há mais de 30 dias (ajustável).
- **Versões do Node pedidas por projetos ficam.** A versão padrão do nvm e todas as que aparecem em arquivos `.nvmrc` são protegidas.
- **Volumes do Docker antes dos contêineres.** Assim um volume de banco ligado a um contêiner parado nunca é apagado sem ter aparecido na lista.
- **Dados de apps instalados nunca entram.** O Sam lê o identificador de cada app em Aplicativos e só oferece pastas que consegue ligar com certeza a um app removido.
- **O motor do Docker não é ligado.** Se o OrbStack ou o Docker Desktop estiver fechado, o Sam avisa e espera você abrir.
- **Nada sai do seu Mac.** O app não faz nenhuma chamada de rede.

## Pastas de projetos

O Sam procura sozinho, até dois níveis abaixo da sua pasta pessoal, a pasta onde estão `eyeofsauron`, `eyeofsauron-2` ou `mordor-2` (os repositórios principais da Field Control), e analisa os repositórios que estão ali. Se os seus projetos ficam em outro lugar, ou em mais de uma pasta, escolha em **Projetos**, no topo da janela. Ali também dá para mudar o prazo para considerar um projeto parado.

Desktop, Documentos e Downloads não entram na busca automática, porque o macOS pediria permissão. Se seus repositórios estiverem lá, adicione a pasta manualmente.

## Permissões do macOS

Algumas pastas só podem ser lidas por apps com **Acesso Total ao Disco**, e o macOS pode perguntar se o Sam pode acessar Downloads ou "dados de outros apps". Quando uma categoria avisar que o macOS bloqueou uma pasta, libere o Sam em **Ajustes → Privacidade e Segurança → Acesso Total ao Disco** (o botão **Como funciona**, no topo, abre essa tela) e meça de novo.

## Desenvolvimento

Precisa de Node 22 e Rust estável.

```bash
npm install
npm run dev      # abre o app com recarga automática
npm test         # testes do Rust e da interface
cargo run --manifest-path src-tauri/Cargo.toml --example scan_report   # mede este Mac no terminal, sem limpar nada
```

Releases saem pelo GitHub Actions: criar uma tag `vX.Y.Z` igual à versão em `package.json`, `src-tauri/Cargo.toml` e `src-tauri/tauri.conf.json` gera o `.dmg` universal num release em rascunho.

Feito com [Tauri 2](https://tauri.app), Svelte 5 e Rust.

## Créditos

- Fontes [**Besley**](https://github.com/indestructible-type/Besley) e [**Atkinson Hyperlegible Next**](https://github.com/googlefonts/atkinson-hyperlegible-next) (SIL Open Font License 1.1), embutidas no app.

## Licença

[MIT](LICENSE).
