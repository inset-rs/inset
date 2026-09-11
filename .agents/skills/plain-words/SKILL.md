---
name: plain-words
description: >-
  Find the words in a document that only its author can resolve, by sending a
  subagent to read it cold and report. Use after writing or rewriting anything a
  later reader has to act on: AGENTS.md, a skill, a PORTING.md, a design note.
---

# Plain words

A document written across a long session picks up words that meant something inside that session and nothing outside it: a term coined while thinking, a nickname for a design, an internal mechanism named where its effect was meant. Rereading does not find them, because each one still carries its meaning for the person who wrote it. A reader who was not there finds them at once.

It matters most where the reader is an agent acting on the text. An agent does not ask what a word means. It guesses, or it copies the shape of whatever example it can see.

## The check

Send a subagent with none of this session's context. It reads and reports; it does not edit.

Give it the file, the audience the file names (a `PORTING.md` names its own: "a reader who knows Rust and only surface Flutter"), and these questions:

- Which terms can you not resolve from this document and the documents it points to? For each, say what you think it means and how sure you are.
- Which sentences would you have to guess at before you could act on them?
- Does any example model the opposite of the rule stated above it?

Then rewrite it yourself: ordinary words, or the term defined where it is first used. Do not answer the report by explaining the term back to the agent. That moves the knowledge into a session that is about to end.

## What it finds

- A word coined while writing. "The house style", where "what every ported type does the same way" was meant.
- A name only this session knows: a type, a design, a shorthand from the conversation that produced the text.
- An internal scenario narrated instead of an effect named. "A state that unmounts after its notifier removes its listener" tells the reader where the code was when it mattered, not what they will see.
- An example that contradicts its own rule. A cold reader copies the example first, so it is worth one question of its own.
