# Capture keys literally and adapt quiet speech safely

Button capture treats every observed key, including Escape and modifier combinations, as a candidate mapping; cancellation is a separate explicit lifecycle action. Automatic gain uses a bounded time-based controller that protects absolute silence, raises distant speech toward a target within the +30 dB ceiling, and soft-limits output instead of hard-clipping it. This keeps mapping intent literal while making far-field speech intelligible without allowing one capture or gain policy to silently dominate the other.
