# CocoDatasetAnnotationsInner


## Properties

Name | Type | Description | Notes
------------ | ------------- | ------------- | -------------
**id** | **int** |  | 
**image_id** | **int** |  | 
**category_id** | **int** |  | 
**bbox** | **List[float]** |  | [optional] 
**segmentation** | **object** |  | [optional] 
**keypoints** | **object** |  | [optional] 
**area** | **float** |  | [optional] 
**iscrowd** | **int** |  | [optional] 

## Example

```python
from moqentra_client.models.coco_dataset_annotations_inner import CocoDatasetAnnotationsInner

# TODO update the JSON string below
json = "{}"
# create an instance of CocoDatasetAnnotationsInner from a JSON string
coco_dataset_annotations_inner_instance = CocoDatasetAnnotationsInner.from_json(json)
# print the JSON string representation of the object
print(CocoDatasetAnnotationsInner.to_json())

# convert the object into a dict
coco_dataset_annotations_inner_dict = coco_dataset_annotations_inner_instance.to_dict()
# create an instance of CocoDatasetAnnotationsInner from a dict
coco_dataset_annotations_inner_from_dict = CocoDatasetAnnotationsInner.from_dict(coco_dataset_annotations_inner_dict)
```
[[Back to Model list]](../README.md#documentation-for-models) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to README]](../README.md)


