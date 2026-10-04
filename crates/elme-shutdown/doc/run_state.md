### Without reload

If an application opts out of `Reload` support, each of the 3 variants (excluding `Reloading`)
will directly mirror their `LifecycleStage` counterparts. e.g:

<!-- TODO: add colour for improved readability -->

<pre>
        <span style="color:cyan;">== CALL STACK ==</span>    LifecycleStage    RunState
   <span style="color:orange;">Application executed</span> ---> --------------|------------
        <span style="color:cyan;">inform_starting()</span> |                |            |
                          |    Startup     | FirstStart |
                          |                |            |
         <span style="color:cyan;">inform_started()</span> | ----- | ------ | ---- | --- |
                          |                |            |
                          |    Running     |    Ready   |
                          |                |            |
                   <span style="color:cyan;">stop()</span> | ----- | ------ | ---- | --- |
        <span style="color:orange;">Teardown starts</span> --->               |            |
                          |    Teardown    |  Stopping  |
      <span style="color:orange;">Teardown finishes</span> --->               |            |
      <span style="color:orange;">Application exits</span> ---> ---------------------------
</pre>

### With reload

The difference between the types is evident when viewed in the context of a `Reload` command:

> **Note:** Teardown start/finish markers excluded in this diagram for clarity

<!-- TODO: add colour for improved readability -->
<pre>
        <span style="color:cyan;">== CALL STACK ==</span>    LifecycleStage    RunState
   <span style="color:orange;">Application executed</span> ---> --------------|------------
        <span style="color:cyan;">inform_starting()</span> |                |            |
                          |    Startup     | FirstStart |
                          |                |            |
         <span style="color:cyan;">inform_started()</span> | ----- | ------ | ---- | --- |
                          |                |            |
                          |    Running     |    Ready   |
                          |                |            |
                 <span style="color:cyan;">reload()</span> | ----- | ------ | ---- | --- |
                          |                |            |
                          |    Teardown    |            |
                          |                |            |
        <span style="color:cyan;">inform_starting()</span> | ----- | ------ |  <b><u>Reloading</u></b> |
                          |                |            |
                          |    Startup     |            |
                          |                |            |
         <span style="color:cyan;">inform_started()</span> | ----- | ------ | ---- | --- |
                          |                |            |
                          |    Running     |    Ready   |
                          |                |            |
                   <span style="color:cyan;">stop()</span> | ----- | ------ | ---- | --- |
                          |                |            |
                          |    Teardown    |  Stopping  |
                          |                |            |
    <span style="color:orange;">Application exits</span> ---> -----------------------------
</pre>

As can be seen:
- The `RunState` variant for a command (e.g. `Reloading`) persists through startup
- The startup variant, `FirstStart` only occurs once
