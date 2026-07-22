# AnnotationResponse


## Properties

Name | Type | Description | Notes
------------ | ------------- | ------------- | -------------
**id** | **str** |  | 
**task_id** | **str** |  | 
**asset_id** | **str** |  | 
**revision** | **int** |  | 
**client_update_id** | **str** |  | 
**actor_id** | **str** |  | 
**payload** | **object** |  | 

## Example

```python
from moqentra_client.models.annotation_response import AnnotationResponse

# TODO update the JSON string below
json = "{}"
# create an instance of AnnotationResponse from a JSON string
annotation_response_instance = AnnotationResponse.from_json(json)
# print the JSON string representation of the object
print(AnnotationResponse.to_json())

# convert the object into a dict
annotation_response_dict = annotation_response_instance.to_dict()
# create an instance of AnnotationResponse from a dict
annotation_response_from_dict = AnnotationResponse.from_dict(annotation_response_dict)
```
[[Back to Model list]](../README.md#documentation-for-models) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to README]](../README.md)


