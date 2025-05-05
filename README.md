[English Version](README.md) | [中文版本](README_zh.md)

# ElfRadio - My Ham Radio Dream, Brought to Life by AI

Hello everyone, I'm VK7KSM, just an ordinary ham radio enthusiast like you. I'm writing this not because I'm some kind of coding guru—quite the contrary, I know next to nothing about code. What I want to share is a story about passion, challenges, a bit of "magic," and the product of this story: ElfRadio.

## How It Began: Lonely Airwaves in the Southern Hemisphere

A few years ago, I emigrated from China to Launceston, a beautiful city on the island of Tasmania, Australia. Life here is simpler, without complex interpersonal relationships, leaving me with plenty of free time after work. I spent a week cramming, found the nearest radio club member, and obtained my Australian amateur radio license (yes, it's the old hands guiding the new ones here, completely different from the organized group exams by the Radio Regulatory Committee in China). I brought along several analog and digital handheld radios (UHF/VHF) and a 100-watt HF base station I had purchased in China. I also set up enormous HF and UV antennas on my roof (surrounded by single-story houses, and my house is on high ground with no obstructions). I imagined myself chatting freely with HAMs worldwide, exploring the wonderful world of radio waves.

But reality quickly threw cold water on my excitement. Tasmania, situated at the edge of the Southern Hemisphere, often suffers from poor HF propagation conditions. Communication with the Northern Hemisphere was weak and fraught with difficulty. And the local UV bands? Mostly farmers checking on their sheep or self-driving tourists exchanging road conditions, all in English thick with heavy accents. My English proficiency, adequate in quiet environments, proved utterly insufficient against the noisy backdrop of radio static and unique accents. I often found myself completely lost, let alone able to participate.

Was my ham radio dream destined to founder like this? I wasn't ready to give up.

## A Spark of Inspiration: When SDR Met AI

I started researching various technologies, hoping for a breakthrough. That's when SDR (Software Defined Radio) and AI (Artificial Intelligence) caught my eye. I tried using a simple SDR receiver with OpenWebRX+ software to listen to signals. Then, a sudden idea struck me—could AI help me understand these signals?

I fed the audio received by OpenWebRX+ into Google's Gemini AI. The result blew me away! Even for voice signals with poor quality, almost indecipherable by ear, Gemini managed to recognize most of the content and accurately transcribe it into text! Even more magically, it could translate the English text into my native Chinese in real-time!

At that moment, a bold idea formed in my mind: If AI could help me "understand," could it also help me "speak"? Could I input Chinese, have AI translate it into English, convert it to speech, and transmit it via the radio? That way, I could communicate barrier-free with local HAMs! A "radio translator" that could break down language barriers and connect the world—this was so cool!

## Realizing My Coding Dream with AI's Help

My career path has included system operations, front-line customer service, software sales, product design, project management, and even running my own company—everything *but* coding. After immigrating to Australia and experiencing the long idle periods during COVID, I started teaching myself programming and frontend development, but progress was slow. However, with the explosion of LLMs like ChatGPT in 2024, I began researching how to use AI for coding. So, I decided to develop an AI-driven learning and operation platform for my personal hobby, amateur radio, to replace the traditionally complex SDR software.

The coding work for this project began on April 21, 2024.

## The Birth of Gemini 2.5, T850, and Skynet

Around that time, Google released the significantly more capable Gemini 2.5. It was said to possess not only exceptional understanding but also the ability to independently write the code for entire software projects! This felt like an opportunity tailor-made for me!

So, I started experimenting with collaborating with Gemini. I poured out all my ideas, requirements, and problems to it. To make the process more efficient and rigorous, I assigned Gemini two "roles":

*   **T850:** My Chief Technology Officer and Primary Architect (the workhorse coder). He is responsible for understanding my requirements, proposing technical solutions, and (most importantly) **writing the code**. My standards for him are extremely high: the code must be precise, efficient, stable, and fully consider the ease of use for a "newbie" user like me.
*   **Skynet:** An independent, extremely critical Technical Consultant (the ruthless overseer), and T850's "nemesis." His task is to scrutinize every solution and every line of code from T850 as if under a microscope, identifying all potential flaws, vulnerabilities, and areas for optimization, and proposing superior alternatives. He is sharp-tongued, gets straight to the point, but his goal is to help us build the best possible product.

Yes, these roles are from "The Terminator," as you might have guessed. T850 is an upgrade from the T800; I designed his upgrade strategy and will release their core directives later.

Our workflow is unique: I propose requirements -> T850 provides solutions/code -> Skynet delivers "merciless" critique -> T850 analyzes feedback and improves -> I make the final decision. This "three-body brainstorming" model is full of sparks but ensures our solutions undergo the most rigorous review.

Incredibly, Gemini (as T850) lived up to expectations! It rapidly built the first version, "Radio Companion," in Python, perfectly implementing the core features I initially envisioned: speech-to-text, translation, and text-to-speech transmission.

But T850 (Gemini) was also honest with me: "Boss, Python is fast for development, but its runtime efficiency isn't great. If we want to implement more cool features and make ElfRadio truly powerful, it's best to rewrite the core code in Rust."

Although I knew nothing about Rust, I trusted T850's professional judgment. So, we decided to rebuild this project, carrying my ham radio dream, using the more powerful Rust language, and officially named it—**ElfRadio**.

## What is ElfRadio?

ElfRadio isn't meant to replace the radio in your hands, nor turn you into a "keyboard warrior" who only clicks a mouse. Its goals are:

*   **To be your intelligent assistant:** Leveraging the power of AI to help you understand difficult signals, break language barriers, and organize QSO information.
*   **To simplify complex operations:** Presenting the often cumbersome settings and operations of traditional radios through a clean, intuitive browser interface, letting you focus on the joy of communication.
*   **To integrate multiple modes:** Bringing together various activities like regular voice QSOs, airband listening, satellite communication, and even future digital modes (via OpenWebRX+ integration) onto a single platform.
*   **To lower the entry barrier:** Allowing newcomers like me, or friends interested in technology but intimidated by traditional equipment and software, to easily take their first steps into the world of amateur radio. Providing a simulated QSO practice feature.
*   **To offer depth for exploration:** While being easy to start with, providing sufficiently rich features and the ability to connect external tools (like OpenWebRX+) to satisfy the exploratory needs of advanced users.

## Planned Features for ElfRadio

1.  AI translation for received and transmitted messages, and analysis of conversation summaries.
2.  Control radio transmission and reception from a computer or mobile browser.
3.  Text-to-speech chat, speech-to-text message reception, plus sending CW messages and (in the future) SSTV images.
4.  Automatically save original audio, STT transcripts, SSTV images, and data information for sent/received messages to local storage (SQLite database and task directories).
5.  All received messages stored in a local SQLite database, enabling AI analysis and search (RAG and vector database).
6.  Also capable of simulated QSO training, airband listening analysis, emergency communication management, Pager message sending (POCSAG), Meshtastic message handling, and gaining more decoding capabilities by connecting to an independent SDR server running OpenWebRX+.
7.  Modular development approach to easily add more features over time.
8.  Configurable AI interfaces, initially built with Google API support, with plans to add APIs for DeepSeek and other advanced LLMs, plus (future) local small models.

## Core Technology Stack

*   **Backend:** **Rust** (Tokio, Axum, SQLx, cpal, serialport-rs, etc.) - Aiming for high performance, stability, and low resource usage. Uses a modular Cargo Workspace structure.
*   **Frontend:** **React + TypeScript + Material UI (MUI) + Vite** - Aiming to build an exquisite, modern, responsive user interface following **Material Design 3 (MD3)** principles.
*   **AI Services:** Prioritizing cloud APIs (Google Gemini, StepFun TTS, OpenAI compatible interfaces like DeepSeek), supporting user-configured API Keys. Designed with a Trait abstraction layer for easy extension.
*   **Deployment:** **Docker / Docker Compose** - Simplifying the deployment and update process.

## Physical Development Environment

*   **Hardware:** A standard home computer (see image below - *assuming image was previously shown*).
*   **Software:** Google Gemini 2.5 Pro (Web UI), Cursor IDE (with Gemini API key and $20/month subscription).

## Virtual Development Team

*   **Boss:** VK7KSM (Project initiator, product manager, requirements provider, final decision-maker, the boss)
*   **T850:** (Played by Gemini) CTO & Primary Architect, workhorse coder, responsible for technical design, code generation, debugging, the ground-level employee.
*   **Skynet:** (Played by Gemini) Independent Technical Consultant, ruthless overseer, responsible for critical reviews, risk assessment, solution optimization suggestions, middle management.

## Code Development Process

1.  **Requirement Definition:** Boss (VK7KSM) proposes product requirements and ideas.
2.  **Solution Planning:** Boss discusses technical feasibility, risks, and implementation plans with T850 and Skynet through multiple rounds of dialogue. Boss makes the final decision.
3.  **Requirement Documentation:** T850 documents the finally confirmed requirements and solutions into detailed development requirement documents and subsequent patches.
4.  **Prompt Generation:** T850 (with Gemini's assistance) breaks down development tasks into fine-grained steps and generates precise English prompts for each step.
5.  **Code Generation:** Boss copies the prompts into Cursor's chat window, and Cursor's embedded AI (Claude/Gemini) automatically generates or modifies Rust/frontend code based on the prompts.
6.  **Compilation & Testing:** Boss uses `cargo check`/`cargo test` or `npm run check`/`npm test` to verify the AI-generated code.
7.  **Debugging & Iteration:** If errors occur, Boss feeds the error messages and Cursor AI's initial solution back to T850 (in the Gemini web UI). T850 analyzes the root cause, evaluates Cursor's solution, and provides a final, more reliable fix prompt. Boss then gives the fix prompt to Cursor for execution. This process repeats until the issue is resolved.
8.  **Version Control:** Boss is responsible for periodically committing compiled and tested code stages to the GitHub repository.

## Pitfalls Encountered During Development

1.  **AI Pleasing Humans:** Gemini's initial development plan was overly aggressive. The first version was in Python and achieved basic interaction. But Gemini told me Python's runtime efficiency was poor, unsuitable for radio operations requiring fast responses, and convinced me to scrap it all and rewrite in Rust. For the frontend, it pushed me towards Svelte + Vite + Tailwind CSS v4 + Headless UI + Svelte Stores. We spent 3 days just trying to get the framework installed correctly, only to find out that both Gemini and Claude fundamentally misunderstood how to use Tailwind v4. It seems AI just recommended what it thought I wanted to hear – the newest, fastest options – without verifying feasibility. AI was just catering to my request for a fast and simple solution.
2.  **Hallucinations:** Both Gemini and Cursor's built-in Claude suffer from severe hallucination problems. The generated code often references non-existent functions and libraries, leading to a flood of errors during `cargo check`. If you ask Cursor's built-in AI to check the errors, it will *never* identify the functions and libraries it just made up. My workaround is to copy the `cargo check` error log and Cursor AI's proposed solution into the web version of Gemini, letting *that* Gemini find the errors and suggest a mostly correct fix.
3.  **Syntax Errors:** Code written by Gemini and Claude frequently contains syntax errors – missing brackets, extra commas, incorrect comment formats. Furthermore, when checking errors, Cursor's built-in Claude fails to spot errors in code *written by Claude*. You have to use Gemini to check Claude's code for errors.
4.  **Gemini Degradation:** When the Gemini 2.5 Pro experimental version was first released, its output quality was extremely high (error-free), and conversation speed was fast (under 20 seconds per response). However, this didn't last. Around April 25th, Gemini 2.5's inference time per response jumped to over 3 minutes, and it frequently produced errors. Cursor's built-in Gemini started generating code with dozens of errors each time, forcing me to switch to Claude for code generation. With the same prompt, Claude's code would have only one or two syntax errors, while Gemini's code was riddled with logic and function definition errors. This degradation issue with Gemini persists even now. As for DeepSeek R1, the code it writes is, frankly, garbage.
5.  **Debugging by Deletion:** Cursor's built-in Gemini and Claude can only generate code based on prompts; they have absolutely no project planning capability. If you ask Cursor's built-in AI to find the cause of errors in the `cargo check` log and fix them, its only solution is to **delete** the code that originally implemented the feature. If you delete all the code, there won't be any errors.
6.  **Context Limits:** The web version of Gemini now supports up to 1 million tokens of context, but even this is insufficient for a software project. Just discussing requirements and uploading reference materials took up 300k tokens. Completing the basic backend functionality reached 700k tokens. Once a conversation exceeds 500k tokens, each message reply takes over 3 minutes, which is agonizingly slow. But starting a new conversation means losing the previous context and model fine-tuning, and retraining/re-feeding all the information from scratch is also tedious. So, the best approach is to **branch** the conversation when it exceeds 400k tokens, creating a clone. Then, you only need to feed it the summary information from the previous conversation to restore its memory, and it requires less fine-tuning to get back into a development state.

## Summary of Development Experience

Using AI to write code is arguably the greatest revolution in human history (a true fifth industrial revolution). It empowers people worldwide who have ideas but lack coding skills to turn their visions into tangible products with the help of AI. With AI, Steve Jobs wouldn't have needed Wozniak to build the Apple prototype, and the Winklevoss twins wouldn't have needed Zuckerberg to write their code. Now, I no longer need to hire expensive programmers to build ElfRadio. Knowing only the basics of programming, I used Gemini to help me structure product requirements, create detailed development documents, guide me on setting up a GitHub account and repository, and instruct me on installing Cursor IDE and configuring the frontend and backend development environments. All of this, for just $20/month!

During actual coding, I found that Cursor's built-in Gemini and Claude couldn't generate more than about 200 lines of code at a time from the web Gemini's prompts without errors (they struggled with complex logic). So, I requested the web Gemini to break down features into smaller steps, generating prompts for only a portion of the functionality at a time. When I copied these smaller prompts to Cursor, the generated code was error-free.

When encountering code errors, I first let Cursor's built-in Gemini and Claude analyze the `cargo check` log and propose a fix. But I don't apply it immediately. Instead, I copy Cursor's proposed fix and the `cargo check` log back into the web version of Gemini, asking it to analyze the feasibility of the solution and suggest better alternatives. The web Gemini then analyzes the situation from a project-wide perspective and provides more rational code modification suggestions. Through two weeks of using Cursor, I've found that its built-in AI can, at best, find syntax errors. It's completely incapable of solving functional or logical errors (its solution is just to delete code). This requires the web-based Gemini, with its broader context, to identify and resolve the problems.

## Current Status and Outlook

ElfRadio has currently completed the setup and initial testing of the Phase 1 backend framework. The codebase includes core functional modules and compiles successfully (all unit tests pass).

The next major step is **frontend development** (based on React + TypeScript + MUI + Vite, following Material Design 3 principles), aiming to quickly implement the core user interface for integration testing with the backend.

Development is ongoing, and I will continue to update the progress. I expect to release a test version with basic functionality in early May. Stay tuned!

## Join Us!

ElfRadio is an open-source project born from passion and developed with AI assistance. Whether you're a seasoned radio operator, a programming novice, or just a tech enthusiast like me, you are welcome to follow, try out, provide suggestions, or even join the development!

Let's connect the world and explore the unknown, together with code and radio waves!

73!

VK7KSM
