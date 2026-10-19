# Acoustic Prosody, Contextual Disambiguation, & Grounded Grammar Spec

- **Feature**: Reconciled Apple-grade Acoustic Prosody (YIN F0 pitch tracking, energy decay, prosodic auto-punctuation) with Deterministic Syntactic Disambiguation (3-token lookaround, spoken symbol parsing, homophone collocations, speech repair pruning, and GroundingGuard whitelisting).
- **Phase**: Streaming Dictation & Input Method Integration (Phase 5 Extension / Phase 2 Expansion)
- **Status**: Ready for Implementation
- **Triage Label**: `ready-for-agent`
- **Branch**: `native-streaming-dictation`

---

## 1. Problem Statement

In continuous desktop streaming dictation, users face four critical friction points:

1. **Forced Verbalization of Punctuation**: Users must awkwardly verbalize *"period"*, *"comma"*, and *"question mark"* aloud. If they speak in a natural conversational flow without vocalizing punctuation, the transcript becomes an unpunctuated run-on block of text.
2. **Spoken Punctuation vs. Literal Symbol Ambiguity**: Polysemous words like *"colon"*, *"period"*, and *"dash"* have dual roles. In expressions like *"the patient was diagnosed with colon cancer"* or *"she studied the colonial period"*, naive engines blindly convert them into symbols (`": cancer"`, `"colonial. "`).
3. **Phonetic Homophone Collisions**: Words with identical acoustic representations (*"their / there / they're"*, *"to / too / two"*, *"cache / cash"*) cannot be separated by acoustic feature extraction alone and require syntactic Part-of-Speech context.
4. **Speech Repairs & Hesitations**: Natural speech contains verbal false starts (*"meet at four, no wait, five pm"*). Literal transcription destroys document clarity unless the abandoned branch is pruned.
5. **Modal Keystroke Locking**: When users type physically on their keyboard while dictating, legacy input methods drop keystrokes or terminate the speech session unexpectedly.

---

## 2. Solution: The Reconciled Three-Tier Architecture

A unified, on-device architecture reconciling **Apple Dictation’s acoustic signal intelligence** with **Narad’s deterministic syntactic heuristics**:

1. **Tier 1 (Acoustic & Prosodic Signal Engine)**:
   - **Real-Time YIN Pitch Tracking ($F_0$) & Energy Decay ($\frac{dE}{dt}$)**: Extracts vocal intonation from 16 kHz audio frames. A terminal pitch drop ($\Delta F_0 \le -35\text{ Hz}$) with silence $\ge 380\text{ ms}$ automatically inserts a period (`.`) and capitalizes the next word; rising uptalk ($\Delta F_0 \ge +30\text{ Hz}$) inserts a question mark (`?`); mid-clause pitch levels with short pauses insert commas (`,`).
   - **Dynamic Joiner Prefix Trie Biasing**: Custom dictionary words (weight 2.5) and active window/IDE symbols (weight 1.0) are indexed into a Trie that injects additive logit boosts directly into the Transducer Joiner layer during modified beam search.
2. **Tier 2 (Live Pre-edit & Dual-Buffer Typing Arbiter)**:
   - **Eager Escape Stripping**: Speaking `"literal colon"` immediately renders `"colon"` in the pre-edit draft, dropping `"literal"` on the fly.
   - **Simultaneous Keystroke Arbitration**: Character typing commits the pre-edit up to the cursor and forwards the key without closing the mic; Backspace pops the trailing word from uncommitted speech.
3. **Tier 3 (Syntactic Disambiguation & Grounded Polish)**:
   - **3-Token Syntactic Lookaround**: Left context (determiners, prepositions, adjectives) and right context (compound nouns) protect literal words from accidental symbolization.
   - **Homophone Collocation Resolver**: Uses local Part-of-Speech bigrams to resolve *their/there/they're* and *cache/cash*.
   - **Speech Repair Pruner**: Strips backtracking markers (*"no wait"*, *"actually no"*), preserving only the corrected clause.
   - **GroundingGuard Whitelisting**: Homophone substitutions count as 0% novel words, ensuring 100% deterministic safety without hallucination risks.

---

## 3. Mathematical & Algorithmic Formulation

### 3.1 YIN Fundamental Frequency ($F_0$) Formulation
For a 16 kHz window $W = 512$ samples (32 ms) and candidate lag $\tau \in [32, 320]$ (50 Hz to 500 Hz):
1. Difference Function:
   $$d_t(\tau) = \sum_{j=1}^{W} (x_j - x_{j+\tau})^2$$
2. Cumulative Mean Normalized Difference Function (CMNDF):
   $$d'_t(\tau) = \begin{cases} 1 & \text{if } \tau = 0 \\ \frac{d_t(\tau)}{\frac{1}{\tau} \sum_{j=1}^\tau d_t(j)} & \text{otherwise} \end{cases}$$
3. Absolute Thresholding ($\theta = 0.15$) + Parabolic Sub-Bin Interpolation yields continuous pitch $F_0 = \frac{f_s}{\tau^*}$.

### 3.2 Joiner Logit Biasing
For active beam hypothesis at Prefix Trie node $q$:
$$\tilde{z}_{t,u}(v) = z_{t,u}(v) + \beta_{\text{boost}}(q, v) \quad \text{for } v \in \text{Children}(q)$$
$$\tilde{z}_{t,u}(w) = z_{t,u}(w) - \gamma_{\text{penalty}} \quad \text{for } w \notin \text{Children}(q) \text{ (when } q \ne q_0\text{)}$$

### 3.3 Boundary Decision Matrix
| Prosodic Cue | Boundary Silence ($\tau_{\text{pause}}$) | Syntactic Context | Resulting Punctuation |
| :--- | :--- | :--- | :--- |
| $\Delta F_0 \le -35\text{ Hz}$, $\frac{dE}{dt} < -0.10\text{ dB/ms}$ | $\ge 380\text{ ms}$ | Complete clause | **Period (`.`)** + Next Token Capitalize |
| $\Delta F_0 \ge +30\text{ Hz}$ | $\ge 250\text{ ms}$ | Any sentence | **Question Mark (`?`)** + Next Token Capitalize |
| $\|\Delta F_0\| < 18\text{ Hz}$ | $160\text{–}380\text{ ms}$ | Continuing phrase | **Comma (`,`)** |
| Preceded by determiner / compound adjective | Any | Noun phrase | **Literal Word** (e.g. *"colon"*, *"period"*) |

---

## 4. Hardware & Resource Budget (NFR Alignment)

- **Streaming Latency**: $\le 30\text{ ms}$ per 50ms chunk (NFR-P-2 headroom).
- **Commit Latency**: $\le 60\text{ ms}$ on pause/stop.
- **Memory (RSS)**: $\le 135\text{ MB}$ total daemon footprint (strictly satisfies NFR-T-4 limit of $<150\text{ MB}$).
- **CPU Utilization**: $\le 10\%$ of 1 core during active speech; $<0.1\%$ during speech pauses (satisfies NFR-P-4).
- **Hallucination Risk**: **0.0%** (mathematically bounded by deterministic `GroundingGuard`).
