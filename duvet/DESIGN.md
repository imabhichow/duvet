## Steps

* [x] Gather all of the file paths for the project and assign an ID
* [x] Compute the linemaps
* [ ] Create file instances
* [x] Register annotation types
* [x] Create annotations and insert regions
* [x] Finalize region maps
* [x] emit annotation maps
* [ ] Use annotation maps to compute status of each annotation with `dyn Type`
* [ ] Create region map + status map


## Examples

### RFC Compliance

* [ ] Extract requirements from document
  * [ ] Create an annotation for each requirement, include the TypeId (MUST, SHOULD, MAY, etc)
* [ ] Extract references in code
  * [ ] Create an annotation for each reference, include the TypeId (citation, test, todo, exception)
  * [ ] Add a relation that links to the requirement AnnotationId with TypeId (reference)
* [ ] Scan each requirement
  * [ ] Set the status for each region that has a relation
  * [ ] Set the status of each requirement

### Invariant pair
* [ ] Extract invariants in code
  * [ ] Create an entity for each invariant
  * [ ] Create an annotation for each invariant reference
* [ ] Mark regions of the code as tests or library
* [ ] Scan each invariant
  * [ ] If each invariant does not have both a test and lib, mark the opposite regions as failed

### Code Coverage
* [ ] Extract all of the covered regions
  * [ ] Create an annotation for each covered region
  * [ ] Create an entity for each test executed
  * [ ] Add a relation that links the region to the test
* [ ] Mark regions of the code as tests or library
* [ ] Create an entity for each function, module, etc
* [ ] Scan each region of code, function, etc
  * [ ] If it doesn't have coverage, mark as failed
  * [ ] Compute stats

### Bolero coverage
* [ ] Create an entity for each fuzz input

