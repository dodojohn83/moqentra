# CocoDatasetImagesInner


## Properties

Name | Type | Description | Notes
------------ | ------------- | ------------- | -------------
**id** | **int** |  | 
**file_name** | **str** |  | 
**width** | **int** |  | 
**height** | **int** |  | 

## Example

```python
from moqentra_client.models.coco_dataset_images_inner import CocoDatasetImagesInner

# TODO update the JSON string below
json = "{}"
# create an instance of CocoDatasetImagesInner from a JSON string
coco_dataset_images_inner_instance = CocoDatasetImagesInner.from_json(json)
# print the JSON string representation of the object
print(CocoDatasetImagesInner.to_json())

# convert the object into a dict
coco_dataset_images_inner_dict = coco_dataset_images_inner_instance.to_dict()
# create an instance of CocoDatasetImagesInner from a dict
coco_dataset_images_inner_from_dict = CocoDatasetImagesInner.from_dict(coco_dataset_images_inner_dict)
```
[[Back to Model list]](../README.md#documentation-for-models) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to README]](../README.md)


