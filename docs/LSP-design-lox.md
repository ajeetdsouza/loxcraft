# Language Server Protocol for lox

- Created: 06 - Feb - 2022
- Updated: 06 - Feb - 2022

## Requirements

- Provide syntax highlighting, code completion and other LSP features for lox.

## TODO (v1)

- [ ] Create a language client extension for lox
  - [ ] Syntax Highlighting
  - [ ] Go To Definition
  - [ ] Code Suggestions / Auto Complete suggestions
- [ ] Create a language server for lox

## Notes

### Language Server Protocol: The Basics

The idea behind the Language Server Protocol (LSP) is to standardize the protocol for how tools and servers communicate, 
so a single Language Server can be re-used in multiple development tools, and tools can support languages with minimal effort.

LSP is a win for both language providers and tooling vendors!

A language server runs as a separate process and development tools communicate with the server using the language protocol using JSON-RPC over TCP.([Spec](https://microsoft.github.io/language-server-protocol/specifications/specification-current/))

The `client` calls the server methods made for certain `events`. 

![Screenshot from 2022-02-06 16-10-04](https://user-images.githubusercontent.com/6604943/152687377-bf77ac27-6916-49dc-ab5f-81535a4c55bf.png)

### Implementation

#### Client-side (VS Code)

The work on the client side should be split on 2 separate extensions based on what I've read up to now.

- [ ] One extension for _Syntax Highlighting_ specifically.
- [ ] One extension to serve as the _Language Server Client_.

I'm really not sure if we can couple these two together and if it's really wise to do so.

Also, note that the _Syntax Highlighting_ can be provided via the server too. So a lot of information can be sent to the server and based on that
information you can dictate what happens on the client. See [here](https://code.visualstudio.com/api/language-extensions/programmatic-language-features).

##### Syntax Highlighting

For _syntax highlighting_ specifically, we would need to prepare a _TextMate_ document of the lox language grammar. [Docs](https://macromates.com/manual/en/language_grammars)

A guide for creating such extension can be found [here](https://code.visualstudio.com/api/language-extensions/syntax-highlight-guide).

I could find a syntax highligter for lox here: https://github.com/danman113/lox-language. We could leverage the same or develop one our own.

##### Language Server Client

Use the VS Code [API](https://code.visualstudio.com/api/references/vscode-api).

There are only few functions that are absolutely necessary, but otherwise it seems like a straight forward thing to implement.

Example: https://code.visualstudio.com/api/language-extensions/language-server-extension-guide#explaining-the-language-client


##### Language Choice

I think the client side for this would most definitely have to be done with _TypeScript_.

I think it's beneficial for the following reasons:

- Easy integration
- MSFT provides some libraries for this with decent documentation
- Great support on the editor for debugging
- Great support for tests

#### Language Server (SDK - Rust/TypeScript)

This is run as a different process by VS Code or any other potential client.

Open-source SDK's that have implemented bindings for a language server in different languages can be seen [here](https://microsoft.github.io/language-server-protocol/implementors/sdks/).

The best one I could find for a Rust based language was `tower-lsp`. [Link](https://github.com/ebkalderon/tower-lsp)

An example to build one: https://code.visualstudio.com/api/language-extensions/language-server-extension-guide#explaining-the-language-server

```

TODO: the specific details for the implementation

```
