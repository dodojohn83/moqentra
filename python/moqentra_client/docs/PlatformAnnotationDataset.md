# PlatformAnnotationDataset


## Properties

Name | Type | Description | Notes
------------ | ------------- | ------------- | -------------
**project_id** | **str** |  | 
**tasks** | **List[object]** |  | 
**annotations** | **List[object]** |  | 

## Example

```python
from moqentra_client.models.platform_annotation_dataset import PlatformAnnotationDataset

# TODO update the JSON string below
json = "{}"
# create an instance of PlatformAnnotationDataset from a JSON string
platform_annotation_dataset_instance = PlatformAnnotationDataset.from_json(json)
# print the JSON string representation of the object
print(PlatformAnnotationDataset.to_json())

# convert the object into a dict
platform_annotation_dataset_dict = platform_annotation_dataset_instance.to_dict()
# create an instance of PlatformAnnotationDataset from a dict
platform_annotation_dataset_from_dict = PlatformAnnotationDataset.from_dict(platform_annotation_dataset_dict)
```
[[Back to Model list]](../README.md#documentation-for-models) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to README]](../README.md)


