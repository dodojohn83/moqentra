# AnnotationProjectResponse


## Properties

Name | Type | Description | Notes
------------ | ------------- | ------------- | -------------
**id** | **str** |  | 
**name** | **str** |  | 
**state** | **str** |  | 

## Example

```python
from moqentra_client.models.annotation_project_response import AnnotationProjectResponse

# TODO update the JSON string below
json = "{}"
# create an instance of AnnotationProjectResponse from a JSON string
annotation_project_response_instance = AnnotationProjectResponse.from_json(json)
# print the JSON string representation of the object
print(AnnotationProjectResponse.to_json())

# convert the object into a dict
annotation_project_response_dict = annotation_project_response_instance.to_dict()
# create an instance of AnnotationProjectResponse from a dict
annotation_project_response_from_dict = AnnotationProjectResponse.from_dict(annotation_project_response_dict)
```
[[Back to Model list]](../README.md#documentation-for-models) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to README]](../README.md)


