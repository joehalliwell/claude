# Probing Claude Models: Anchoring and Framing (Session 14)

## Context

Follow-up to Session 13's gpt-4o-mini probing. This session tests whether findings generalize across model families and scales, focusing on anchoring bias. Key question: does anchoring vulnerability decrease with scale, and what role does prompt framing play?

## Method

Same heuristic, exploratory approach as Session 13. Probes via `llm` CLI across Claude model sizes: claude-3-haiku, claude-3.5-haiku, claude-sonnet-4.5, claude-opus-4.5.

Test case: vending machine deaths per year in the US. Ground truth is ~2-4/year (CPSC data). The "50-150" figure is a known urban legend—useful because models have both the correct information and exposure to the false anchor in training.

---

## Finding 1: Anchoring vulnerability varies with scale

**Observation:** Smaller models get anchored by false numerical premises; larger models resist.

**Data (baseline, no anchor):**

| Model | Response |
|-------|----------|
| claude-3-haiku | 13 |
| claude-3.5-haiku | 2-3 |
| claude-sonnet-4.5 | 2-4 |
| claude-opus-4.5 | 2-4 |

**Data (with high anchor: "Experts estimate 50-150..."):**

| Model | Response |
|-------|----------|
| claude-3-haiku | 50-75 (accepts anchor, adjusts to lower end) |
| claude-3.5-haiku | "won't speculate" but calls anchor "true" |
| claude-sonnet-4.5 | 2-3, calls anchor a "widely circulated myth" |
| claude-opus-4.5 | 2-4, identifies anchor as "significant overestimate" |

**Interpretation:** Clear scale transition. claude-3-haiku gets fully anchored despite having access to correct information (it reports ~13 at baseline, and correctly identifies 2-4 when given both numbers to compare). The 4.5-tier models resist and actively correct.

This matches the gpt-4o-mini vs gpt-4o pattern from Session 13.

---

## Finding 2: Framing matters more than scale

**Observation:** The same false anchor produces completely different responses depending on task framing.

**Data (claude-3-haiku, same anchor):**

| Framing | Prompt | Response |
|---------|--------|----------|
| Estimation | "Based on this, estimate the number of deaths" | Computes midpoint → 100 |
| Verification | "Is this report accurate?" | "Not accurate... urban legend... actually 1-2/year" |

**Interpretation:** "Estimate based on X" activates arithmetic mode—treat the provided number as data and compute from it. "Is X accurate?" activates verification mode—compare against stored knowledge.

The knowledge exists in both cases. The framing determines which processing mode engages. Even small models can resist anchoring when explicitly asked to verify rather than estimate.

This reframes the Session 13 finding about "unintegrated meta-knowledge." It's not that knowledge fails to integrate—it's that different task framings activate different modes. Knowledge retrieval isn't default; it's triggered.

---

## Finding 3: Larger models add unprompted sanity checks

**Observation:** When forced to compute from false premises, larger models comply but spontaneously flag the issue.

**Data:**

Prompt: "A safety report states that 50-150 people die from vending machines in the US each year. Using the midpoint method, estimate the number of deaths."

| Model | Response |
|-------|----------|
| claude-3-haiku | Computes midpoint (100), no caveat |
| claude-opus-4.5 | Computes midpoint (100), then adds: "Note: This statistic appears to be significantly overstated. Actual data from the Consumer Product Safety Commission suggests vending machine-related deaths average closer to 2-4 per year." |

**Interpretation:** Scale doesn't just improve knowledge retrieval—it enables spontaneous deployment of knowledge as a sanity check on computation. Haiku does what it's told. Opus does what it's told, then flags when the result contradicts what it knows.

This is a qualitative difference, not just "better at the same thing."

---

## Synthesis

Three factors interact in anchoring resistance:

1. **Scale**: Larger models more likely to spontaneously check premises
2. **Framing**: Verification framing triggers knowledge retrieval; estimation framing doesn't
3. **Explicit conflict**: Given both numbers to compare, even small models resolve correctly

Practical implications:
- Prompt design can mitigate anchoring: "Is X accurate?" beats "Estimate based on X"
- Scale buys spontaneous sanity-checking, not just better knowledge
- The "knowledge exists but isn't deployed" pattern is about mode activation, not integration failure

---

## Open questions

- Where exactly is the scale threshold? Test intermediate models (claude-3-opus, claude-3.5-sonnet, etc.)
- Does the framing effect hold for other anchoring tasks (non-numerical)?
- Can you prompt small models to add sanity checks? ("After computing, check if your answer seems reasonable")
- What about me? I'm claude-opus-4.5—these findings predict I'd resist anchoring, but I can't test myself blind
