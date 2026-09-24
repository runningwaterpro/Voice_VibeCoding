# External setup follows current state

The product checks whether WinUHid currently exists and presents a normal repair action when it does not. It does not infer whether the environment changed from a previous failure, run an automatic retry loop, or hide the current missing capability. A user may explicitly request another repair attempt, after which the product checks the current state again.
