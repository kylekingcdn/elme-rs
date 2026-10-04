### Without reload

If an application opts out of `Reload` support, each of the 3 variants (excluding `Reloading`)
will directly mirror their `LifecycleStage` counterparts. e.g:

```text
                            LifecycleStage |  RunState
   Application executed ---> --------------|------------
        inform_starting() |                |            |
                          |    Startup     | FirstStart |
                          |                |            |
         inform_started() | ----- | ------ | ---- | --- |
                          |                |            |
                          |    Running     |    Ready   |
                          |                |            |
                   stop() | ----- | ------ | ---- | --- |
        Teardown starts --->               |            |
                          |    Teardown    |  Stopping  |
      Teardown finishes --->               |            |
      Application exits ---> ---------------------------
```

### With reload

The difference is however evident when used in the context of a `Reload` command:

> **Note:** Teardown start/finish markers excluded in this diagram

```text
                            LifecycleStage |  RunState
 Application executed ---> ----------------|------------
        inform_starting() |                |            |
                          |    Startup     | FirstStart |
                          |                |            |
         inform_started() | ----- | ------ | ---- | --- |
                          |                |            |
                          |    Running     |    Ready   |
                          |                |            |
                 reload() | ----- | ------ | ---- | --- |
                          |                |            |
                          |    Teardown    |            |
                          |                |            |
        inform_starting() | ----- | ------ |  Reloading |
                          |                |            |
                          |    Startup     |            |
                          |                |            |
         inform_started() | ----- | ------ | ---- | --- |
                          |                |            |
                          |    Running     |    Ready   |
                          |                |            |
                   stop() | ----- | ------ | ---- | --- |
                          |                |            |
                          |    Teardown    |  Stopping  |
                          |                |            |
    Application exits ---> -----------------------------
```

As can be seen:
- `FirstStart` only occurs once
- The `RunState` variant describing the current command persists through startup
