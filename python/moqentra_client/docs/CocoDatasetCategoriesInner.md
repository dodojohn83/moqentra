# CocoDatasetCategoriesInner


## Properties

Name | Type | Description | Notes
------------ | ------------- | ------------- | -------------
**id** | **int** |  | 
**name** | **str** |  | 
**supercategory** | **str** |  | [optional] 

## Example

```python
from moqentra_client.models.coco_dataset_categories_inner import CocoDatasetCategoriesInner

# TODO update the JSON string below
json = "{}"
# create an instance of CocoDatasetCategoriesInner from a JSON string
coco_dataset_categories_inner_instance = CocoDatasetCategoriesInner.from_json(json)
# print the JSON string representation of the object
print(CocoDatasetCategoriesInner.to_json())

# convert the object into a dict
coco_dataset_categories_inner_dict = coco_dataset_categories_inner_instance.to_dict()
# create an instance of CocoDatasetCategoriesInner from a dict
coco_dataset_categories_inner_from_dict = CocoDatasetCategoriesInner.from_dict(coco_dataset_categories_inner_dict)
```
[[Back to Model list]](../README.md#documentation-for-models) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to README]](../README.md)


