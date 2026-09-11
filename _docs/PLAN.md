# Personal Memory Tool - Project Plan

## 1. Project Vision

Build a local-first personal memory tool that acts as a memory layer for the computer.

The core experience:

> Press one global shortcut anywhere -> capture the most useful thing on screen -> save it instantly -> local AI remembers, organizes, connects, and resurfaces it later.

The tool is not intended to be another generic notes app. Its differentiation is the combination of:

- Extremely fast capture
- One global shortcut
- Context-aware screen/selection capture
- Persistent personal memory
- Local GPU-hosted AI
- Semantic and natural-language retrieval
- Automatic connections between old and new information
- Contextual resurfacing

---

## 2. Problem

Important information is constantly encountered and then lost:

- A useful quote in a book
- A random idea
- An article
- A useful web page
- A video
- A task
- Something from a conversation
- A useful piece of information visible on screen

Existing tools often require the user to decide where and how to save something before capturing it.

This project should minimize that friction.

The principle is:

> Capture first. Organize later.

---

## 3. Target User

Primary user:

- One individual
- Heavy computer user
- Reads books/articles and consumes online content
- Frequently has ideas
- Wants to save information quickly
- Uses a desktop computer and browser
- Values privacy and local processing
- Has access to a local GPU

This is initially a personal tool, not a multi-user SaaS product.

---

## 4. Core Product Principle

### One Shortcut

The primary interaction should be a single global keyboard shortcut.

The user should not need to:

- Open the application
- Choose a notebook
- Choose a folder
- Select a content type
- Add tags
- Fill out metadata

Instead:

1. User sees something useful.
2. User presses the global shortcut.
3. Tool captures the relevant screen/selection context.
4. Local processing determines what is useful.
5. Original capture is preserved.
6. Item is stored.
7. AI organization happens automatically or asynchronously.

---

## 5. Capture System

### Primary capture flow

Global shortcut -> capture current selection/on-screen content -> save.

The system should automatically determine the most useful representation.

Possible inputs:

- Selected text
- Visible text
- Browser content
- Screenshot
- URL
- Application context
- Clipboard content
- Other visible information

The system should decide what combination is useful.

For example:

A book quote may become:

- Original quote
- Screenshot/context
- Book/page/source information when available

An article may become:

- Relevant text
- URL
- Page title

An idea may become:

- Captured text
- Application/source
- Timestamp

The user should not have to manually classify the capture.

---

## 6. Raw Capture Preservation

Important decision:

The original captured item should remain exactly as captured.

AI-generated metadata should never replace the original.

Each capture should therefore have:

- Original content
- Capture timestamp
- Source/context when available
- AI-generated metadata separately

This prevents AI processing from corrupting the user's original memory.

---

## 7. Content Types

The system should support:

- Book quotes
- Ideas
- Notes
- Articles
- Web links
- Videos/posts
- Conversations/messages
- Tasks
- General information

The user does not necessarily select the type.

The local AI can classify it after capture.

---

## 8. Local AI

AI should be primarily local.

The system should support a model hosted on the user's local GPU.

Potential AI responsibilities:

### Classification

Determine what type of item was captured.

### Metadata

Generate useful metadata such as:

- Topics
- Keywords
- Entities
- Possible source
- Book/author information when available
- Short description

### Embeddings

Generate embeddings locally for semantic retrieval.

### Retrieval

Understand natural-language searches such as:

> "Find that quote about discipline I saved months ago."

### Connections

Identify relationships between captures.

Example:

> "This idea is related to three things you saved previously."

### Personal assistant

Answer questions using the user's stored knowledge.

### Resurfacing

Identify old information that may be relevant to current activity.

---

## 9. Personal Knowledge Assistant

The AI assistant should eventually support questions such as:

- "What did I save about RAG?"
- "Find that quote about discipline."
- "What books have I saved quotes from about leadership?"
- "Have I had this idea before?"
- "Show me things related to this project."
- "What were my previous thoughts about this?"
- "Connect this idea with things I've saved before."
- "Help me develop this idea using my previous notes."

The assistant should prioritize the user's own stored information rather than generic model knowledge.

---

## 10. Search

Search should support multiple methods.

### Exact search

Find matching words or phrases.

### Semantic search

Find conceptually related information even when the exact words differ.

### Natural-language search

Example:

> "That quote about doing difficult things that I saved last year."

### Filters

Potential filters:

- Type
- Date
- Book
- Author
- Topic
- Source
- Application
- Tags

### Related content

Every item should be able to show related items.

---

## 11. Resurfacing

The system should proactively bring useful information back.

Three important modes:

### Daily resurfacing

Show selected older captures that may be worth revisiting.

### Contextual resurfacing

When the user is working on something, surface relevant older captures.

### Unexpected connections

Example:

> "You saved this six months ago. It appears related to what you're working on now."

This should eventually become one of the major differentiators.

---

## 12. Main UI

The home screen should combine several functions rather than being a simple notes list.

Potential dashboard:

### Inbox

Recently captured items.

### Memory

Organized personal knowledge.

### Search

Natural-language and exact search.

### Connections

Related ideas and knowledge graph.

### Today

Useful resurfaced memories.

### AI Assistant

Chat with personal knowledge.

The UI should remain simple despite the system being powerful.

---

## 13. Desktop Application

Initial platform:

- Desktop application
- Browser extension

The desktop application owns the global shortcut and local knowledge system.

The browser extension makes browser capture easier and provides additional context such as:

- URL
- Page title
- Selected text
- Browser metadata

---

## 14. Proposed Architecture

High-level architecture:

```text
                 USER
                   |
                   v
          Global Keyboard Shortcut
                   |
                   v
             Capture Engine
                   |
          +--------+--------+
          |                 |
          v                 v
     Screen/Selection    Context
       Extraction       Detection
          |                 |
          +--------+--------+
                   |
                   v
             Raw Capture
                   |
                   v
             Local Storage
                   |
                   v
          Background AI Pipeline
                   |
       +-----------+-----------+
       |           |           |
       v           v           v
   Classifier   Embeddings   Metadata
       |           |           |
       +-----------+-----------+
                   |
                   v
          Personal Knowledge DB
                   |
       +-----------+-----------+
       |           |           |
       v           v           v
    Search     Connections   Resurfacing
       |           |           |
       +-----------+-----------+
                   |
                   v
          Personal AI Assistant