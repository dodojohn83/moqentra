
# OutboxEvent


## Properties

Name | Type
------------ | -------------
`id` | string
`aggregate_type` | string
`aggregate_id` | string
`event_type` | string
`payload` | object

## Example

```typescript
import type { OutboxEvent } from ''

// TODO: Update the object below with actual values
const example = {
  "id": null,
  "aggregate_type": null,
  "aggregate_id": null,
  "event_type": null,
  "payload": null,
} satisfies OutboxEvent

console.log(example)

// Convert the instance to a JSON string
const exampleJSON: string = JSON.stringify(example)
console.log(exampleJSON)

// Parse the JSON string back to an object
const exampleParsed = JSON.parse(exampleJSON) as OutboxEvent
console.log(exampleParsed)
```

[[Back to top]](#) [[Back to API list]](../README.md#api-endpoints) [[Back to Model list]](../README.md#models) [[Back to README]](../README.md)


