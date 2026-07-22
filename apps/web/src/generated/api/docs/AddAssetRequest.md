
# AddAssetRequest


## Properties

Name | Type
------------ | -------------
`name` | string
`object_key` | string
`digest` | string
`size` | number
`media_type` | string
`metadata` | object

## Example

```typescript
import type { AddAssetRequest } from ''

// TODO: Update the object below with actual values
const example = {
  "name": null,
  "object_key": null,
  "digest": null,
  "size": null,
  "media_type": null,
  "metadata": null,
} satisfies AddAssetRequest

console.log(example)

// Convert the instance to a JSON string
const exampleJSON: string = JSON.stringify(example)
console.log(exampleJSON)

// Parse the JSON string back to an object
const exampleParsed = JSON.parse(exampleJSON) as AddAssetRequest
console.log(exampleParsed)
```

[[Back to top]](#) [[Back to API list]](../README.md#api-endpoints) [[Back to Model list]](../README.md#models) [[Back to README]](../README.md)


