# Narad Streaming Dictation

A local, on-device streaming voice-to-text dictation engine, input method bridge, and companion HUD for Linux.

## Language

**Pre-edit**:
Transient, uncommitted text displayed with an underline directly in the focused application's text widget.
_Avoid_: Composition string, draft text, floating buffer

**Committed Text**:
Final text injected permanently into the document buffer or target widget.
_Avoid_: Inserted text, finalized buffer

**Acoustic Prosody**:
Vocal pitch intonation ($F_0$) and energy decay dynamics extracted from raw audio to infer clause boundaries and punctuation.
_Avoid_: Audio features, voice tone, inflection

**Syntactic Lookaround**:
Deterministic n-token window inspection of preceding and succeeding words to disambiguate polysemous symbols and homophones.
_Avoid_: Text heuristics, context parser

**Eager Escape**:
A spoken prefix (such as "literal" or "word") that immediately strips the prefix and forces literal noun output in pre-edit.
_Avoid_: Force literal, escape command

**Speech Repair**:
A conversational false-start where the speaker backtracks using verbal markers ("no wait", "actually no") to correct their utterance.
_Avoid_: Scratch that, mistake correction

**Constituent Phrase Scope**:
The grammatical boundary window (e.g. compound numbers, currency, noun phrases) over which a speech repair marker excises an abandoned spoken fragment.
_Avoid_: Repair window, pop range

**Clause Commit**:
Automatic intermediate commit of a completed sentence or clause triggered by a natural pause without ending the dictation session.
_Avoid_: Auto-pause, silence stop

**Pre-edit Ceiling**:
A threshold (soft ceiling of ~20 words / 120 characters) at which completed sentences are committed to prevent UI rendering degradation and horizontal scroll jitter during continuous speech.
_Avoid_: Buffer limit, max preedit

**Punctuation Precedence**:
The deterministic rule whereby an explicit spoken punctuation command atomically supersedes and overwrites any prior tentatively placed prosodic mark on the preceding token.
_Avoid_: Punctuation override, mark priority

**In-Stream Mode Transition**:
A spoken mode change event ("code mode", "email mode") that triggers an immediate clause commit of prior speech under the outgoing mode before formatting subsequent speech under the incoming mode.
_Avoid_: Dynamic mode switch, mid-sentence mode

**Grounding Guard**:
A deterministic gate checking length ratios and novel-word counts to prevent ungrounded alterations to user speech.
_Avoid_: Hallucination filter, safety check

**Word-Level Rewind**:
An input method action where physical Backspace intercepts and pops the trailing recognized word from an active pre-edit hypothesis rather than deleting a single character.
_Avoid_: Character backspace, token delete

**Prefix Trie Biasing**:
A runtime trie indexing contextual hotwords and active application identifiers to additively boost logit scores in the Transducer Joiner layer without mutating base model weights.
_Avoid_: Hotword list, keyword booster

**Energy Decay Fallback**:
An acoustic heuristic that uses short-time log energy slope ($\frac{dE}{dt} < -0.15\text{ dB/ms}$) and extended silence pause to infer terminal sentence punctuation when pitch harmonics are unvoiced or degraded by noise.
_Avoid_: Energy VAD, silence punctuation
