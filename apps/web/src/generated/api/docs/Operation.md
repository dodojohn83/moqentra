
# Operation


## Properties

Name | Type
------------ | -------------
`id` | string
`state` | string
`resourceRef` | string
`error` | [ProblemDetails](ProblemDetails.md)

## Example

```typescript
import type { Operation } from ''

// TODO: Update the object below with actual values
const example = {
  "id": null,
  "state": null,
  "resourceRef": null,
  "error": null,
} satisfies Operation

console.log(example)

// Convert the instance to a JSON string
const exampleJSON: string = JSON.stringify(example)
console.log(exampleJSON)

// Parse the JSON string back to an object
const exampleParsed = JSON.parse(exampleJSON) as Operation
console.log(exampleParsed)
```

[[Back to top]](#) [[Back to API list]](../README.md#api-endpoints) [[Back to Model list]](../README.md#models) [[Back to README]](../README.md)


