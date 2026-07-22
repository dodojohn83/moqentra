# CocoDataset


## Properties

Name | Type | Description | Notes
------------ | ------------- | ------------- | -------------
**info** | **object** |  | [optional] 
**images** | [**List[CocoDatasetImagesInner]**](CocoDatasetImagesInner.md) |  | 
**categories** | [**List[CocoDatasetCategoriesInner]**](CocoDatasetCategoriesInner.md) |  | 
**annotations** | [**List[CocoDatasetAnnotationsInner]**](CocoDatasetAnnotationsInner.md) |  | 

## Example

```python
from moqentra_client.models.coco_dataset import CocoDataset

# TODO update the JSON string below
json = "{}"
# create an instance of CocoDataset from a JSON string
coco_dataset_instance = CocoDataset.from_json(json)
# print the JSON string representation of the object
print(CocoDataset.to_json())

# convert the object into a dict
coco_dataset_dict = coco_dataset_instance.to_dict()
# create an instance of CocoDataset from a dict
coco_dataset_from_dict = CocoDataset.from_dict(coco_dataset_dict)
```
[[Back to Model list]](../README.md#documentation-for-models) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to README]](../README.md)


