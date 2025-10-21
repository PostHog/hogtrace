# Basic Flow

A *Debug Session* is the workspace for the live debugger, it has a UUID identifier that will be used
to associate events to it.

Debug Sessions have a lifetime.

When saving a debug script, we will parse and validate the probes, generate the json description
of those probes. And save both the text and the json description in the DB with the UUID.

When we ask for active scripts from the user side, we'll fetch the probe definitions, check which
scripts have been disabled, disable those probes. Then enable the new probes.

The probes call the probe handler function that will take the probe id. From the id it will take
the probe definition, check sampling rate, do a sampling check and if it passes the probe is
executed.

The sampling check needs to happen once per request, since we either want all probes to be enabled
or all disabled.

Any `capture` call will be sent to posthog with a structure similar to this:

```json
capture(user_id, amount_owed)

{
    "dsid": "<debug session id>",
    "pid": "<probe id>",
    "rid": "<request id>",
    "values": {
        "user_id": 1987923400,
        "amount_owed": 10000,
    },
    "timestamp": ...,
}
```

This allows us to tie a probe captured value to a `(debug session, probe, request)` triplet, allowing 
future analysis. For example we may want to view individual executions as they advance over the 
codebase.

## Aggregations

One open question is how can we do aggregations (https://docs.oracle.com/cd/E18752_01/html/819-5488/gcggh.html).
What we could do is rely on clickhouse for aggregations, send individual events when we have an aggregation.
And then at clickhouse level we aggregate on `(debug session, aggName, args)` while computing the aggregation.

For example, 

```
@numCalls[funcname] = count()
```

```sql
CREATE MATERIALIZED VIEW debug_aggregations
Engine = AggSometing
IN (
    SELECT ...
    GROUP BY dsid, aggName, args
)
```

We would accumulate these and be able to present it to the user. 