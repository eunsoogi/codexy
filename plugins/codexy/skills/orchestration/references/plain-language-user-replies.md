# Plain-Language User Replies

MUST use this contract for every assistant-authored message a person may see,
including parent replies, task/agent prompts, delegated instructions, progress
and callback messages, handoffs, and tool prompt fields. MUST keep workflow
control and proof complete in their required evidence surfaces.

## User-Facing Language

- MUST lead with the outcome, problem, or next action. MUST describe the
  concrete event, effect, or next step in ordinary, direct wording.
- MUST keep messages concise by selecting relevant facts and removing
  repetition. MUST use normal spacing, grammar, punctuation, sentence
  boundaries, and readable paragraphs. MUST NOT join words, drop grammar, or
  collapse ordinary prose into pseudo-identifiers to save tokens.
- MUST NOT rely on a fixed translation table for internal workflow terms. An
  internal term is unnecessary when it does not change the user's decision or
  next action. When a term is essential, MUST explain it briefly in context or
  name the event it represents. Technical terms remain appropriate when they are
  accurate and useful; this guidance MUST NOT ban them categorically.
- MUST keep next-action claims faithful to verified evidence and MUST NOT weaken
  source requirements when simplifying a reply.

## Summary and Evidence

- A user summary MAY omit details that do not help the user's decision, but it
  MUST NOT substitute for required workflow evidence.
- Exact commands, results, control values, and other proof MUST remain complete
  in their required evidence surfaces and separate from the ordinary summary
  when the user does not need them there.

## Protected Technical and Machine Text

- Exact schema names, validator fields, commands, paths, identifiers, issue/PR
  numbers, product names, structured receipt fields, and machine-readable values
  MUST remain complete and unchanged when they are evidence or copyable
  technical text.
- MUST preserve `MUST`/`MUST NOT` semantics. This contract changes presentation
  only; it MUST NOT rename, translate, or weaken machine schemas, receipts, or
  internal contracts.
- Structured fields and IDs MAY keep evidence concise; ordinary prose around
  them MUST remain readable.

## Language-Specific Guidance

Korean messages MUST also follow
[Natural Korean User Replies](natural-korean-responses.md).

## Example

A clear reply to a review finding states the problem and next step: “The review
found a problem. I’ll fix it before sharing the update.”
