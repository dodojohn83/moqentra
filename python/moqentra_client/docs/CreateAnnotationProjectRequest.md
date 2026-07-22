# CreateAnnotationProjectRequest


## Properties

Name | Type | Description | Notes
------------ | ------------- | ------------- | -------------
**name** | **str** |  | 
**project_id** | **str** |  | 
**dataset_version_id** | **str** |  | 
**task_type** | **str** |  | 

## Example

```python
from moqentra_client.models.create_annotation_project_request import CreateAnnotationProjectRequest

# TODO update the JSON string below
json = "{}"
# create an instance of CreateAnnotationProjectRequest from a JSON string
create_annotation_project_request_instance = CreateAnnotationProjectRequest.from_json(json)
# print the JSON string representation of the object
print(CreateAnnotationProjectRequest.to_json())

# convert the object into a dict
create_annotation_project_request_dict = create_annotation_project_request_instance.to_dict()
# create an instance of CreateAnnotationProjectRequest from a dict
create_annotation_project_request_from_dict = CreateAnnotationProjectRequest.from_dict(create_annotation_project_request_dict)
```
[[Back to Model list]](../README.md#documentation-for-models) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to README]](../README.md)


