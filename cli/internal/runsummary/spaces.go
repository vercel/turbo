package runsummary

import (
	"encoding/json"
	"fmt"
	"sync"

	"github.com/mitchellh/cli"
	"github.com/vercel/turbo/cli/internal/ci"
	"github.com/vercel/turbo/cli/internal/client"
)

const runsEndpoint = "/v0/spaces/%s/runs"
const runsPatchEndpoint = "/v0/spaces/%s/runs/%s"
const tasksEndpoint = "/v0/spaces/%s/runs/%s/tasks"

// spaceRequest contains all the information for a single request to Spaces
type spaceRequest struct {
	method  string
	url     string
	body    interface{}
	makeURL func(self *spaceRequest, r *spaceRun) error // Should set url on self
	onDone  func(self *spaceRequest, response []byte)   // Handler for when request completes
}

func (req *spaceRequest) error(msg string) error {
	return fmt.Errorf("[%s] %s: %s", req.method, req.url, msg)
}

type spacesClient struct {
	requests chan *spaceRequest
	errors   []error
	api      *client.APIClient
	ui       cli.Ui
	run      *spaceRun
	wg       sync.WaitGroup
	spaceID  string
	enabled  bool
}

type spaceRun struct {
	ID      string
	URL     string
	created chan struct{} // a signal that the run has completed
}

func newSpacesClient(spaceID string, api *client.APIClient, ui cli.Ui) *spacesClient {
	return &spacesClient{
		api:      api,
		ui:       ui,
		spaceID:  spaceID,
		enabled:  spaceID != "",
		requests: make(chan *spaceRequest), // TODO: give this a size based on tasks
		// Set a default, empty one here, so we'll have something downstream and not a segfault
		run: &spaceRun{
			created: make(chan struct{}, 1),
		},
	}
}

// Start receiving and processing requests in 8 goroutines
// There is an additional marker (protected by a mutex) that indicates
// when the first request is done. All other requests are blocked on that one.
// This first request is the POST /run request. We need to block on it because
// the response contains the run ID from the server, which we need to construct the
// URLs of subsequent requests.
func (c *spacesClient) start() {
	if !c.enabled {
		return
	}

	// Start an immediately invoked go routine that listens for requests coming in from a channel
	pending := []*spaceRequest{}
	firstRequestStarted := false

	// Create a labeled statement so we can break out of the for loop more easily

	// Setup a for loop that goes infinitely until we break out of it
FirstRequest:
	for {
		// A select statement that can listen for messages from multiple channels
		select {
		// listen for new requests coming in
		case req := <-c.requests:
			if req == nil {
				continue
			}

			// Make the first request right away in a goroutine,
			// queue all other requests. When the first request is done,
			// we'll get a message on the other channel and break out of this loop
			if !firstRequestStarted {
				firstRequestStarted = true
				go c.dequeueRequest(req)
			} else {
				pending = append(pending, req)
			}
			// Wait for c.run.created channel to be closed and:
		case <-c.run.created:
			// 1. flush pending requests
			for _, req := range pending {
				go c.dequeueRequest(req)
			}

			// 2. break out of the forever loop.
			break FirstRequest
		}
	}

	// and then continue listening for more requests as they come in until the channel is closed
	for req := range c.requests {
		go c.dequeueRequest(req)
	}
}

func (c *spacesClient) makeRequest(req *spaceRequest) {
	if !c.enabled {
		return
	}

	// The runID is required for POST task requests and PATCH run request
	// so we have to construct it lazily for those requests.
	// We construc this first in makeRequest, because if makeURL fails, it's likely
	// because we don't have a runID, which means that the first POST run failed. By checking this
	// up front, we can avoid duplicate error messages for things like missing spaceID / linking.
	//
	// TODO: the purpose of this check up front is just to make sure runID is available for the
	// requests that need it. Maybe we can leverage the c.run.created channel or another channel for
	// this so it's more explicit?
	if req.makeURL != nil {
		if err := req.makeURL(req, c.run); err != nil {
			c.errors = append(c.errors, err)
			return
		}
	}

	if c.spaceID == "" {
		c.errors = append(c.errors, req.error("No spaceID found"))
		return
	}

	if !c.api.IsLinked() {
		c.errors = append(c.errors, req.error("Repo is not linked to a Space. Run `turbo link --target=spaces` first"))
		return
	}

	// We only care about POST and PATCH right now
	if req.method != "POST" && req.method != "PATCH" {
		c.errors = append(c.errors, req.error(fmt.Sprintf("Unsupported method %s", req.method)))
		return
	}

	payload, err := json.Marshal(req.body)
	if err != nil {
		c.errors = append(c.errors, req.error(fmt.Sprintf("Failed to create payload: %s", err)))
		return
	}

	// Make the request
	var resp []byte
	var reqErr error
	if req.method == "POST" {
		resp, reqErr = c.api.JSONPost(req.url, payload)
	} else if req.method == "PATCH" {
		resp, reqErr = c.api.JSONPatch(req.url, payload)
	} else {
		c.errors = append(c.errors, req.error("Unsupported request method"))
	}

	if reqErr != nil {
		c.errors = append(c.errors, req.error(fmt.Sprintf("%s", reqErr)))
		return
	}

	// Call the onDone handler if there is one
	if req.onDone != nil {
		req.onDone(req, resp)
	}
}

func (c *spacesClient) createRun(rsm *Meta) {
	if !c.enabled {
		return
	}

	c.queueRequest(&spaceRequest{
		method: "POST",
		url:    fmt.Sprintf(runsEndpoint, c.spaceID),
		body:   newSpacesRunCreatePayload(rsm),

		// handler for when the request finishes. We set the response into a struct on the client
		// because we need the run ID and URL from the server later.
		onDone: func(req *spaceRequest, response []byte) {
			if response == nil {
				return
			}

			if err := json.Unmarshal(response, c.run); err != nil {
				c.errors = append(c.errors, req.error(fmt.Sprintf("Error unmarshaling response: %s", err)))
			}

			// close the run.created channel, because all other requests are blocked on it
			close(c.run.created)
		},
	})
}

func (c *spacesClient) postTask(task *TaskSummary) {
	if !c.enabled {
		return
	}

	c.queueRequest(&spaceRequest{
		method: "POST",
		makeURL: func(self *spaceRequest, run *spaceRun) error {
			if run.ID == "" {
				return fmt.Errorf("No Run ID found to post task %s", task.TaskID)
			}
			self.url = fmt.Sprintf(tasksEndpoint, c.spaceID, run.ID)
			return nil
		},
		body: newSpacesTaskPayload(task),
	})
}

func (c *spacesClient) finishRun(rsm *Meta) {
	if !c.enabled {
		return
	}

	c.queueRequest(&spaceRequest{
		method: "PATCH",
		makeURL: func(self *spaceRequest, run *spaceRun) error {
			if run.ID == "" {
				return fmt.Errorf("No Run ID found to send PATCH request")
			}
			self.url = fmt.Sprintf(runsPatchEndpoint, c.spaceID, run.ID)
			return nil
		},
		body: newSpacesDonePayload(rsm.RunSummary),
	})
}

// queueRequest adds the given request to the requests channel and increments the waitGroup counter
func (c *spacesClient) queueRequest(req *spaceRequest) {
	c.wg.Add(1)
	c.requests <- req
}

// dequeueRequest makes the request in a go routine and decrements the waitGroup counter
func (c *spacesClient) dequeueRequest(req *spaceRequest) {
	defer c.wg.Done()
	c.makeRequest(req)
}

// Cloe will wait for all requests to finish
func (c *spacesClient) Close() {
	close(c.requests) // close out the channel since there should be no more requests
	c.wg.Wait()       // wait for all requests to finish
}

type spacesClientSummary struct {
	ID      string `json:"id"`
	Name    string `json:"name"`
	Version string `json:"version"`
}

type spacesRunPayload struct {
	StartTime      int64               `json:"startTime,omitempty"`      // when the run was started
	EndTime        int64               `json:"endTime,omitempty"`        // when the run ended. we should never submit start and end at the same time.
	Status         string              `json:"status,omitempty"`         // Status is "running" or "completed"
	Type           string              `json:"type,omitempty"`           // hardcoded to "TURBO"
	ExitCode       int                 `json:"exitCode,omitempty"`       // exit code for the full run
	Command        string              `json:"command,omitempty"`        // the thing that kicked off the turbo run
	RepositoryPath string              `json:"repositoryPath,omitempty"` // where the command was invoked from
	Context        string              `json:"context,omitempty"`        // the host on which this Run was executed (e.g. Github Action, Vercel, etc)
	Client         spacesClientSummary `json:"client"`                   // Details about the turbo client
	GitBranch      string              `json:"gitBranch"`
	GitSha         string              `json:"gitSha"`
	User           string              `json:"originationUser,omitempty"`
}

// spacesCacheStatus is the same as TaskCacheSummary so we can convert
// spacesCacheStatus(cacheSummary), but change the json tags, to omit local and remote fields
type spacesCacheStatus struct {
	// omitted fields, but here so we can convert from TaskCacheSummary easily
	Local     bool   `json:"-"`
	Remote    bool   `json:"-"`
	Status    string `json:"status"` // should always be there
	Source    string `json:"source,omitempty"`
	TimeSaved int    `json:"timeSaved"`
}

type spacesTask struct {
	Key          string            `json:"key,omitempty"`
	Name         string            `json:"name,omitempty"`
	Workspace    string            `json:"workspace,omitempty"`
	Hash         string            `json:"hash,omitempty"`
	StartTime    int64             `json:"startTime,omitempty"`
	EndTime      int64             `json:"endTime,omitempty"`
	Cache        spacesCacheStatus `json:"cache,omitempty"`
	ExitCode     int               `json:"exitCode,omitempty"`
	Dependencies []string          `json:"dependencies,omitempty"`
	Dependents   []string          `json:"dependents,omitempty"`
	Logs         string            `json:"log"`
}

func newSpacesRunCreatePayload(rsm *Meta) *spacesRunPayload {
	startTime := rsm.RunSummary.ExecutionSummary.startedAt.UnixMilli()
	context := "LOCAL"
	if name := ci.Constant(); name != "" {
		context = name
	}

	return &spacesRunPayload{
		StartTime:      startTime,
		Status:         "running",
		Command:        rsm.synthesizedCommand,
		RepositoryPath: rsm.repoPath.ToString(),
		Type:           "TURBO",
		Context:        context,
		GitBranch:      rsm.RunSummary.SCM.Branch,
		GitSha:         rsm.RunSummary.SCM.Sha,
		User:           rsm.RunSummary.User,
		Client: spacesClientSummary{
			ID:      "turbo",
			Name:    "Turbo",
			Version: rsm.RunSummary.TurboVersion,
		},
	}
}

func newSpacesDonePayload(runsummary *RunSummary) *spacesRunPayload {
	endTime := runsummary.ExecutionSummary.endedAt.UnixMilli()
	return &spacesRunPayload{
		Status:   "completed",
		EndTime:  endTime,
		ExitCode: runsummary.ExecutionSummary.exitCode,
	}
}

func newSpacesTaskPayload(taskSummary *TaskSummary) *spacesTask {
	startTime := taskSummary.Execution.startAt.UnixMilli()
	endTime := taskSummary.Execution.endTime().UnixMilli()

	return &spacesTask{
		Key:          taskSummary.TaskID,
		Name:         taskSummary.Task,
		Workspace:    taskSummary.Package,
		Hash:         taskSummary.Hash,
		StartTime:    startTime,
		EndTime:      endTime,
		Cache:        spacesCacheStatus(taskSummary.CacheSummary), // wrapped so we can remove fields
		ExitCode:     *taskSummary.Execution.exitCode,
		Dependencies: taskSummary.Dependencies,
		Dependents:   taskSummary.Dependents,
		Logs:         string(taskSummary.GetLogs()),
	}
}
