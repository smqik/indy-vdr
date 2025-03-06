/*
Copyright Scoir Inc. All Rights Reserved.

SPDX-License-Identifier: Apache-2.0
*/

package vdr

/*
#cgo LDFLAGS: -lindy_vdr
#include "libindy_vdr.h"
#include <stdlib.h>

extern void submitRequest(ErrorCode err, const char* response);
typedef void (*submitRequestWrapper)(ErrorCode err, const char* response);

extern void refresh(ErrorCode err);
typedef void (*refreshWrapper)(ErrorCode err);

extern void status(ErrorCode err, const char* response);
typedef void (*statusWrapper)(ErrorCode err, const char* response);

*/
import "C"

import (
	"bytes"
	"crypto/ed25519"
	"crypto/sha256"
	"encoding/base64"
	"encoding/json"
	"fmt"
	"io"
	"io/ioutil"
	"log"
	"sync"
	"unsafe"

	"github.com/hyperledger/indy-vdr/wrappers/golang/crypto"
	"github.com/hyperledger/indy-vdr/wrappers/golang/identifiers"
	"github.com/mr-tron/base58"
	"github.com/pkg/errors"
)

type Handle C.int64_t

type Client struct {
	pool        Handle
	genesisTxns []byte
}

// New creates an Indy IndyVDR client connected to the Indy distributed ledger identified by the genesis file
// provided as a reader.
func New(genesis io.ReadCloser) (*Client, error) {

	txns, err := ioutil.ReadAll(genesis)
	if err != nil {
		return nil, fmt.Errorf("reading genesis file failed: %w", err)
	}

	params := map[string]interface{}{
		"transactions": string(txns),
	}

	d, err := json.Marshal(params)
	if err != nil {
		return nil, fmt.Errorf("formatting json params to indy failed: %w", err)
	}

	var pool C.int64_t
	cparams := C.CString(string(d))
	result := C.indy_vdr_pool_create(cparams, &pool)
	C.free(unsafe.Pointer(cparams))
	if result != 0 {
		return nil, fmt.Errorf("open indy pool failed. (Indy error code: [%v])", result)
	}

	out := &Client{
		pool:        Handle(pool),
		genesisTxns: txns,
	}
	return out, nil
}

// Genesis returns the genesis file of the network to which this client is connected
func (r *Client) Genesis() []byte {
	return r.genesisTxns
}

// Close shuts down the connection and frees all resources form the indy distributed ledger
func (r *Client) Close() error {
	result := C.indy_vdr_pool_close(C.int64_t(r.pool))
	if result != 0 {
		return fmt.Errorf("close indy pool failed: (Indy error code: [%v])", result)
	}

	return nil
}

type SubmitResponse struct {
	ErrorCode int
	Response  string
}

var submitRequestCh = make(chan SubmitResponse, 1)
var submitRequestLock = sync.Mutex{}

//export submitRequestCb
func submitRequestCb(cb_id C.CallbackId, err C.ErrorCode, response *C.char) {
	msg := SubmitResponse{
		ErrorCode: int(err),
		Response:  C.GoString(response),
	}

	submitRequestCh <- msg
}

func (r *Client) GetRichSchema(id string) (*ReadReply, error) {
	fmt.Println("reading rich shema from ledger !!!")
	var nymreq *C.int64_t
	var none *C.char
	none = C.CString("BuGZVAtnRDcQvxNYckm1CW")
	//var none32 C.int32_t = -1 // seq_no
	//var none64 C.int64_t = -1 // timestamp
	//cdid := C.CString("efuihyy83yrfhuyef")
	result := C.indy_vdr_build_get_rich_schema_object_by_metadata_request(none, C.CString("sch"), C.CString("TestSchema"), C.CString("1.0"), nymreq)

	//result = C.indy_vdr_build_get_nym_request(none, cdid, none32, none64, &nymreq)
	//C.free(unsafe.Pointer(cdid))
	if result != 0 {
		return nil, fmt.Errorf("invalid get rich schema request: (Indy error code: [%v])", result)
	}
	defer C.indy_vdr_request_free(*nymreq)
	res, err := r.submitReadRequest(*nymreq)
	if err != nil {
		fmt.Println("Error while reading schema: ", err)
		return nil, nil
	}
	fmt.Println("Result is ------> ", res)
	return res, nil
}

// Submit is used to send prepared read requests to the ledger where the request parameter is the JSON-formatted payload.
func (r *Client) Submit(request []byte) (*ReadReply, error) {
	fmt.Println("\nSubmitting custom request...")
	var cusreq C.int64_t
	cjson := C.CString(string(request))
	result := C.indy_vdr_build_custom_request(cjson, &cusreq)
	fmt.Println("\nResult is --> ", result)
	C.free(unsafe.Pointer(cjson))
	if result != 0 {
		var errMsg *C.char
		C.indy_vdr_get_current_error(&errMsg)
		defer C.free(unsafe.Pointer(errMsg))
		fmt.Printf("invalid custom request: (Indy error code: [%s])", C.GoString(errMsg))
		return nil, fmt.Errorf("invalid custom request: (Indy error code: [%s])", C.GoString(errMsg))
	}
	defer C.indy_vdr_request_free(cusreq)

	return r.submitReadRequest(cusreq)
}

// GetNym fetches the NYM transaction associated with a DID
// FIXME: Expose optional seq_no and timestamp to get specific NYM versions
// on did:indy compliant ledgers
func (r *Client) GetNym(did string) (*ReadReply, error) {
	var nymreq C.int64_t
	var none *C.char
	var none32 C.int32_t = -1 // seq_no
	var none64 C.int64_t = -1 // timestamp
	cdid := C.CString(did)
	result := C.indy_vdr_build_get_nym_request(none, cdid, none32, none64, &nymreq)
	C.free(unsafe.Pointer(cdid))
	if result != 0 {
		return nil, fmt.Errorf("invalid get nym request: (Indy error code: [%v])", result)
	}
	defer C.indy_vdr_request_free(nymreq)

	return r.submitReadRequest(nymreq)
}

func (r *Client) MultiSign() (*ReadReply, error) {
	fmt.Println("\nSubmitting custom request...")
	var cusreq C.int64_t
	request := `
	{"operation":{"type":"1","dest":"LQVcTQajEfHFgC7dJeWJ6R3uBsqZrSdp9rTzv344p4A","verkey":"Cx5yErN9Eroiwshar79aaFUvn8BRah7i79pXjCWg1XTQ"},"identifier":"8yjHBnH5QL6EuBxy1cnyFvjAggHprw6R2fQYtcE3kYR2","endorser":"8yjHBnH5QL6EuBxy1cnyFvjAggHprw6R2fQYtcE3kYR2","reqId":2374687226,"protocolVersion":2,"signature":"2zSGUMB38i7LUqD3wRdeEjymdav7cw1YYYjVwdqGZT7Hub7canZChoZJprjqETHj4ohEhRsE5cDTuGUj9d1kn8V1"}`
	cjson := C.CString(string(request))
	result := C.indy_vdr_build_custom_request(cjson, &cusreq)
	fmt.Println("\nResult is --> ", result)
	C.free(unsafe.Pointer(cjson))
	if result != 0 {
		var errMsg *C.char
		C.indy_vdr_get_current_error(&errMsg)
		defer C.free(unsafe.Pointer(errMsg))
		fmt.Printf("invalid custom request: (Indy error code: [%s])", C.GoString(errMsg))
		return nil, fmt.Errorf("invalid custom request: (Indy error code: [%s])", C.GoString(errMsg))
	}
	defer C.indy_vdr_request_free(cusreq)
	//r.submitReadRequest(cusreq)
	//return r.submitReadRequest(cusreq)
	// var nymreq C.int64_t

	// // var none *C.char
	// // var none32 C.int32_t = -1 // seq_no
	// // var none64 C.int64_t = -1 // timestamp
	// 	cjson := C.int64_t(len([]byte(request)))
	//cjson := C.CString(request)
	cdid := C.CString("8yjHBnH5QL6EuBxy1cnyFvjAggHprw6R2fQYtcE3kYR2")
	byt := new(bytes.Buffer)
	// sig, _ := sign.Sign(cursbyte)
	byt.Write([]byte("36HcHAhk62mpoBRPX6RgnhBCDKknWZrWhA9wCYKqzAyEn8YmiCd91L5FSwE4yi24Urhqus7cmUmdeAynufFfEqtH"))
	// intVal := int64(binary.BigEndian.Uint64(request))
	// length := len(request)
	// lenbytes, _ := json.Marshal(length)
	// intValLen := (*C.int64_t)(unsafe.Pointer(&lenbytes))
	sigBytes := []byte("36HcHAhk62mpoBRPX6RgnhBCDKknWZrWhA9wCYKqzAyEn8YmiCd91L5FSwE4yi24Urhqus7cmUmdeAynufFfEqtH")
	data8 := (*C.uint8_t)(unsafe.Pointer(&sigBytes))
	buf := C.ByteBuffer{len: C.int64_t(len([]byte("36HcHAhk62mpoBRPX6RgnhBCDKknWZrWhA9wCYKqzAyEn8YmiCd91L5FSwE4yi24Urhqus7cmUmdeAynufFfEqtH"))), data: data8}
	resultIs := C.indy_vdr_request_set_multi_signature(cusreq, C.CString("8yjHBnH5QL6EuBxy1cnyFvjAggHprw6R2fQYtcE3kYR2"), buf)
	C.free(unsafe.Pointer(cdid))
	if result != 0 {
		fmt.Printf("invalid multi-sign request: (Indy error code: [%v])", resultIs)
		return nil, fmt.Errorf("invalid multi-sign request: (Indy error code: [%v])", resultIs)
	}
	fmt.Println("Result is: : ", cusreq)
	r.submitReadRequest(cusreq)
	//defer C.indy_vdr_request_free(nymreq)
	return nil, nil

	//return r.submitReadRequest(resultIs)
}

// GetTxnAuthorAgreement fetches the current ledger Transaction Author Agreement
func (r *Client) GetTxnAuthorAgreement() (*ReadReply, error) {
	var taareq C.int64_t
	var none *C.char
	result := C.indy_vdr_build_get_txn_author_agreement_request(none, none, &taareq)
	if result != 0 {
		return nil, fmt.Errorf("invalid get taa request: (Indy error code: [%v])", result)
	}
	defer C.indy_vdr_request_free(taareq)

	return r.submitReadRequest(taareq)
}

// GetAcceptanceMethodList fetches the current ledger Acceptance Methods List (for the TAA)
func (r *Client) GetAcceptanceMethodList() (*ReadReply, error) {
	var amlreq C.int64_t
	var none *C.char
	var zero C.int64_t
	result := C.indy_vdr_build_get_acceptance_mechanisms_request(none, zero, none, &amlreq)
	if result != 0 {
		return nil, fmt.Errorf("invalid get aml request: (Indy error code: [%v])", result)
	}
	defer C.indy_vdr_request_free(amlreq)

	return r.submitReadRequest(amlreq)
}

// GetEndpoint fetches the registered endpoint for a DID
func (r *Client) GetEndpoint(did string) (*ReadReply, error) {
	return r.GetAttrib(did, "endpoint")
}

type RefreshResponse struct {
	ErrorCode int
}

var refreshCh = make(chan RefreshResponse, 1)
var refreshLock = sync.Mutex{}

//export refreshCb
func refreshCb(cb_id C.CallbackId, err C.ErrorCode) {
	msg := RefreshResponse{
		ErrorCode: int(err),
	}

	refreshCh <- msg
}

// RefreshPool retrieves the current pool transactions for the ledger
func (r *Client) RefreshPool() error {
	refreshLock.Lock()
	defer refreshLock.Unlock()

	result := C.indy_vdr_pool_refresh(C.int64_t(r.pool), C.refreshWrapper(C.refresh), 0)
	if result != 0 {
		var errMsg *C.char
		C.indy_vdr_get_current_error(&errMsg)
		defer C.free(unsafe.Pointer(errMsg))
		return fmt.Errorf("refresh pool failed: (Indy error code: [%v] %s)", result, C.GoString(errMsg))
	}

	res := <-refreshCh
	if res.ErrorCode > 0 {
		return fmt.Errorf("refresh pool error result: (Indy error code: [%v])", res.ErrorCode)
	}

	return nil
}

type StatusResponse struct {
	ErrorCode int
	Response  string
}

var statusCh = make(chan StatusResponse, 1)
var statusLock = sync.Mutex{}

//export statusCb
func statusCb(cb_id C.CallbackId, err C.ErrorCode, response *C.char) {
	msg := StatusResponse{
		ErrorCode: int(err),
		Response:  C.GoString(response),
	}

	statusCh <- msg
}

// GetPoolStatus fetches the current status and node list of the distributed ledger
func (r *Client) GetPoolStatus() (*PoolStatus, error) {
	statusLock.Lock()
	defer statusLock.Unlock()

	result := C.indy_vdr_pool_get_status(C.int64_t(r.pool), C.statusWrapper(C.status), 0)
	if result != 0 {
		return nil, fmt.Errorf("get pool status failed: (Indy error code: [%v])", result)
	}

	res := <-statusCh
	if res.ErrorCode > 0 {
		return nil, fmt.Errorf("error from pool status: (Indy error code: [%v])", res.ErrorCode)
	}

	ps := &PoolStatus{}
	err := json.Unmarshal([]byte(res.Response), ps)
	if err != nil {
		return nil, fmt.Errorf("unmarshaling pool status failed: %w", err)
	}

	return ps, nil
}

// GetAttrib fetches the attribute from the raw field of the provided DID
func (r *Client) GetAttrib(did, raw string) (*ReadReply, error) {
	attribreq := NewRawAttribRequest(did, raw, did)
	d, err := json.Marshal(attribreq)
	if err != nil {
		return nil, fmt.Errorf("marhsal indy attr request failed: (%w)", err)
	}

	response, err := r.Submit(d)
	if err != nil {
		return nil, fmt.Errorf("unable to submit indy get attr request. (%w)", err)
	}

	return response, nil

}

// AddHandle adds a handle for the provided DID
func (r *Client) AddHandle(did, handle string) (*ReadReply, error) {
	attribreq := NewHandleRequest(did, handle, did)
	//attribreq := NewRawAttribRequest(did, handle, did)
	// d, err := json.Marshal(attribreq)
	// if err != nil {
	// 	return nil, fmt.Errorf("marhsal indy handle request failed: (%w)", err)
	// }

	// seed, err := identifiers.ConvertSeed("arunsteward000000000000000000000")
	// if err != nil {
	// 	log.Fatalln(err)
	// }
	privkey := ed25519.PrivateKey{83, 24, 37, 254, 51, 97, 175, 13, 103, 181, 98, 200, 220, 149, 82, 91, 168, 248, 137, 159, 62, 64, 106, 26, 63, 223, 97, 89, 255, 217, 210, 50, 86, 138, 225, 111, 121, 207, 185, 123, 29, 87, 230, 86, 49, 186, 1, 122, 38, 82, 253, 119, 209, 129, 66, 189, 78, 56, 182, 222, 17, 96, 138, 159}
	//	privkey := ed25519.NewKeyFromSeed(seed)
	pubkey := privkey.Public().(ed25519.PublicKey)

	fmt.Println("---- Private Key ---- ", base58.Encode(privkey))
	fmt.Println("---- Public Key ---- ", base58.Encode(pubkey))
	//	pub, priv, _ := ed25519.GenerateKey()
	sign := crypto.NewSigner(pubkey, privkey)
	response, err := r.SubmitWrite(attribreq, sign)
	if err != nil {
		fmt.Println("Error is: ", err)
		panic(err)
	}
	fmt.Println("Response is ---> ", response)
	return nil, nil
	// response, err := r.Submit(d)
	// if err != nil {
	// 	return nil, fmt.Errorf("unable to submit indy add handle request. (%w)", err)
	// }

	// return response, nil

}

// GetSchema returns the schema definition defined by schemaID on the Indy distributed ledger
func (r *Client) GetSchema(schemaID string) (*ReadReply, error) {
	var schemareq C.int64_t
	var none *C.char
	cschema := C.CString(schemaID)
	result := C.indy_vdr_build_get_schema_request(none, cschema, &schemareq)
	C.free(unsafe.Pointer(cschema))
	if result != 0 {
		return nil, fmt.Errorf("invalid get schema request: (Indy error code: [%v])", result)
	}
	defer C.indy_vdr_request_free(schemareq)

	return r.submitReadRequest(schemareq)

}

// GetCredDef returns the credential definition defined by credDefID on the Indy distributed ledger
func (r *Client) GetCredDef(credDefID string) (*ReadReply, error) {
	var credDefReqNo C.int64_t
	var none *C.char
	cschema := C.CString(credDefID)
	result := C.indy_vdr_build_get_cred_def_request(none, cschema, &credDefReqNo)
	C.free(unsafe.Pointer(cschema))
	if result != 0 {
		return nil, fmt.Errorf("invalid get credential definition request: (Indy error code: [%v])", result)
	}
	defer C.indy_vdr_request_free(credDefReqNo)

	return r.submitReadRequest(credDefReqNo)

}

// GetAuthRules fetches all AUTH rules for the ledger
func (r *Client) GetAuthRules() (*ReadReply, error) {
	return r.GetTxnTypeAuthRule("", "", "")
}

// TODO:  figure out why "*" doesn't work as a wildcard for field
// GetTxnTypeAuthRule fetches the AUTH rule for a specific transaction type and action
func (r *Client) GetTxnTypeAuthRule(typ, action, field string) (*ReadReply, error) {
	var authReq *Request
	switch action {
	case AuthActionEdit:
		authReq = NewAuthEditRuleRequest(typ, field)
	case AuthActionAdd:
		authReq = NewAuthAddRuleRequest(typ, field)
	default:
		authReq = NewAuthRulesRequest()
	}

	d, err := json.Marshal(authReq)
	if err != nil {
		return nil, fmt.Errorf("marhsal indy auth rule request failed: (%w)", err)
	}

	response, err := r.Submit(d)
	if err != nil {
		return nil, fmt.Errorf("unable to submit indy auth rule request. (%w)", err)
	}

	return response, nil
}

func (r *Client) submitReadRequest(reqID C.int64_t) (*ReadReply, error) {
	submitRequestLock.Lock()
	defer submitRequestLock.Unlock()
	result := C.indy_vdr_pool_submit_request(C.int64_t(r.pool), reqID, C.submitRequestWrapper(C.submitRequest), 0)
	if result != 0 {
		var errMsg *C.char
		C.indy_vdr_get_current_error(&errMsg)
		defer C.free(unsafe.Pointer(errMsg))
		return nil, fmt.Errorf("unable to submit request: (Indy error code: [%v] %s)", result, C.GoString(errMsg))
	}
	res := <-submitRequestCh
	if res.ErrorCode > 0 {
		var errMsg *C.char
		C.indy_vdr_get_current_error(&errMsg)
		defer C.free(unsafe.Pointer(errMsg))
		return nil, fmt.Errorf("error from submitted request: (Indy error code: [%v] %s)", result, C.GoString(errMsg))
	}

	fmt.Println("\n\nRes is ----> ", res)
	rply, err := parseReadReply(res.Response)
	if err != nil {
		return nil, err
	}

	return rply, nil
}

func (r *Client) submitWriteRequest(reqID C.int64_t) (*WriteReply, error) {
	submitRequestLock.Lock()
	defer submitRequestLock.Unlock()
	result := C.indy_vdr_pool_submit_request(C.int64_t(r.pool), reqID, C.submitRequestWrapper(C.submitRequest), 0)
	if result != 0 {
		var errMsg *C.char
		C.indy_vdr_get_current_error(&errMsg)
		defer C.free(unsafe.Pointer(errMsg))
		return nil, fmt.Errorf("unable to submit request: (Indy error code: [%v] %s)", result, C.GoString(errMsg))
	}
	res := <-submitRequestCh
	if res.ErrorCode > 0 {
		var errMsg *C.char
		C.indy_vdr_get_current_error(&errMsg)
		defer C.free(unsafe.Pointer(errMsg))
		return nil, fmt.Errorf("error from submitted request: (Indy error code: [%v] %s)", result, C.GoString(errMsg))
	}

	rply, err := parseWriteReply(res.Response)
	if err != nil {
		return nil, err
	}

	return rply, nil
}

// SubmitWrite is used to send prepared write requests to the ledger where the req parameter is the JSON-formatted payload.
// the signer defined a service capable of signing a message that is allowed to be written to the ledger.
func (r *Client) SubmitWrite(req *Request, signer Signer) (*WriteReply, error) {
	d, _ := json.MarshalIndent(req, " ", "")
	m := map[string]interface{}{}
	_ = json.Unmarshal(d, &m)

	ser, err := SerializeSignature(m)
	if err != nil {
		return nil, errors.Wrap(err, "unable to generate signature")
	}

	sig, err := signer.Sign([]byte(ser))
	if err != nil {
		return nil, errors.Wrap(err, "unable to sign write request")
	}

	req.Signature = base58.Encode(sig)
	fmt.Println("SIgnature of request is: ", req.Signature)
	request, err := json.MarshalIndent(req, " ", "")
	if err != nil {
		return nil, errors.Wrap(err, "unable to marshal write request")
	}

	var cusreq C.int64_t
	cjson := C.CString(string(request))
	result := C.indy_vdr_build_custom_request(cjson, &cusreq)
	C.free(unsafe.Pointer(cjson))
	if result != 0 {
		var errMsg *C.char
		C.indy_vdr_get_current_error(&errMsg)
		defer C.free(unsafe.Pointer(errMsg))
		return nil, fmt.Errorf("invalid custom writerequest: (Indy error code: [%s])", C.GoString(errMsg))
	}
	defer C.indy_vdr_request_free(cusreq)

	return r.submitWriteRequest(cusreq)
}

func (r *Client) AddNYM() (*ReadReply, error) {
	seed, err := identifiers.ConvertSeed("amitsteward000000000000000000000")
	if err != nil {
		log.Fatalln(err)
	}
	privkey := ed25519.NewKeyFromSeed(seed)
	pubkey := privkey.Public().(ed25519.PublicKey)

	fmt.Println("---- Private Key ---- ", base58.Encode(privkey))
	fmt.Println("---- Public Key ---- ", base58.Encode(pubkey))
	//	pub, priv, _ := ed25519.GenerateKey()
	sign := crypto.NewSigner(pubkey, privkey)
	//did := did()
	diddoc := `
	{"@context":["https://www.w3.org/ns/did/v1","https://w3id.org/security/suites/ed25519-2020/v1","https://w3id.org/security/suites/jws-2020/v1","https://w3id.org/security/bbs/v1"],"diddoc":{"@context":["https://www.w3.org/ns/did/v1","https://w3id.org/security/suites/ed25519-2020/v1","https://w3id.org/security/suites/jws-2020/v1","https://w3id.org/security/bbs/v1"],"id":"did:fox:8yjHBnH5QL6EuBxy1cnyFvjAggHprw6R2fQYtcE3kYR2","authentication":["did:fox:8yjHBnH5QL6EuBxy1cnyFvjAggHprw6R2fQYtcE3kYR2#key-ed25519-1"],"assertionMethod":["did:fox:8yjHBnH5QL6EuBxy1cnyFvjAggHprw6R2fQYtcE3kYR2#key-bbs-1","did:fox:8yjHBnH5QL6EuBxy1cnyFvjAggHprw6R2fQYtcE3kYR2#key-ed25519-1"],"keyAgreement":["did:fox:8yjHBnH5QL6EuBxy1cnyFvjAggHprw6R2fQYtcE3kYR2#key-bbs-1","did:fox:8yjHBnH5QL6EuBxy1cnyFvjAggHprw6R2fQYtcE3kYR2#key-ed25519-1","did:fox:8yjHBnH5QL6EuBxy1cnyFvjAggHprw6R2fQYtcE3kYR2#key-ecp384-1"],"verificationMethod":[{"id":"did:fox:8yjHBnH5QL6EuBxy1cnyFvjAggHprw6R2fQYtcE3kYR2#key-bbs-1","type":"BbsBlsSignature2020","publicKeyBase58":"eyJQb2ludEcyIjp7ImN1cnZlIjo2LCJlbGVtZW50IjoiRksvQ1ZsSDdYQWZIRXZHaUF1MTNtQ1g3ZkZLM0FqZkFWL1ovektwdGh0TEhVL1p1WEVyYzhqVXFraHF0TnNtdURLRUI0U2hQTHZwcE5VSFJOYW85cGJ4VWI1Z3B5TFc4b0xSRjdaUTNBaitIVFMvTno4SkMvOFNPb1d5aFlvR1VFQk0xZXZWejFIUGJHbXBodHc2R083eUJYcExldUc3dlNQNkRPeTJnQVVVSElVaEJWVHNvT2tkV1g5Z0VhdkxuRm56c1ZhaC9IM3hPZERlZVBFUDI0a0xhOE1jQ055SkhlQlhrQnhHMkFWelRpNzJKTkNjMS9FSklvTlMvY2p2MyJ9fQ","publicKeyJwk":{}},{"id":"did:fox:8yjHBnH5QL6EuBxy1cnyFvjAggHprw6R2fQYtcE3kYR2#key-ed25519-1","type":"JsonWebKey2020","publicKeyJwk":{"crv":"OKP","kty":"Ed25519","x":"InNZMGVyL3o2VWkrOHVONGpaR3JaSUU2SWhnSXplY3dBZk5zcFpjVUJ3b009Ig"}},{"id":"did:fox:8yjHBnH5QL6EuBxy1cnyFvjAggHprw6R2fQYtcE3kYR2#key-ecp384-1","type":"JsonWebKey2020","publicKeyJwk":{"crv":"EC","kty":"P-384","x":"UXqUixEFMC0HJicYFNyT9ACU0v62RSNR_JsnrgepSyZQdGFxBoyEDLBuDdPI_ssy","y":"u_xtAiBk_lqTCreD6p1CcQ3lzJm3QqoPdwShNtSB8xfgxVCvosLAAAR5r1j9qVDe"}}]}
}
	  `

	err = r.CreateNym("8yjHBnH5QL6EuBxy1cnyFvjAggHprw6R2fQYtcE3kYR2", "Cx5yErN9Eroiwshar79aaFUvn8BRah7i79pXjCWg1XTQ", "101", "BuGZVAtnRDcQvxNYckm1CW", diddoc, sign)
	if err != nil {
		fmt.Println("Error in updating NYM of steward: ", err)
		return nil, err
	}
	// fmt.Println("Method specific DID is: ", did.String())
	// err = r.CreateNym(did.DIDVal.MethodSpecificID, "GCGZneQDHnWE4G3ug3j2VXrjW8iawWaE7Yc1zMZss6Bz", "", "BuGZVAtnRDcQvxNYckm1CW", diddoc, sign)
	// //reply, err := r.CreateRichSchema(id, "TestContext", "1.0", content, sign)
	// if err != nil {
	// 	fmt.Println("Error in creating NYM: ", err)
	// 	return nil, err
	// }
	//	fmt.Println("Response is: ", reply)
	//fmt.Println("Rich schema added successfully: ", txnId)
	fmt.Println("--------------------------------")
	return nil, nil
	//if err
}

func did() *identifiers.DID {
	didInfo := new(identifiers.MyDIDInfo)
	didInfo.DID = "AY3dFAPWoL4YpRYJNTqieQQGcVxpFb88CnxPrsmzFhF"
	didInfo.MethodName = "fox"
	didInfo.PublicKey = []byte("GCGZneQDHnWE4G3ug3j2VXrjW8iawWaE7Yc1zMZss6Bz")
	didInfo.Cid = false
	did, _ := identifiers.CreateDID(didInfo)
	return did
}

func (r *Client) EndorseTransaction() {
	authorDID := "2sz51n1iryuv5shFN3PuNfDR1FEsPXt9yWvfHc4hbabA"
	authorPubKey := "3r27cGC7WpsttqRmDx9DLGEsHodF8uE72b7rAfw7xzfM"
	authorprivkey := "ZNkVENWpjRYHiPNsdx3hdcY3FwFH496MaqUdUu9bwYvmttoLFETPz1jXzzuYMnZq1zW83qtc42Kky9xxryRW3Fq"
	// seed, err := identifiers.ConvertSeed("")
	// if err != nil {
	// 	log.Fatalln(err)
	// }
	// privkey := ed25519.NewKeyFromSeed(seed)
	// pubkey := privkey.Public().(ed25519.PublicKey)

	// fmt.Println("---- Private Key ---- ", base58.Encode(privkey))
	// fmt.Println("---- Public Key ---- ", base58.Encode(pubkey))
	priv, _ := base58.Decode(authorprivkey)
	pub, _ := base58.Decode("3r27cGC7WpsttqRmDx9DLGEsHodF8uE72b7rAfw7xzfM")

	//	pub, priv, _ := ed25519.GenerateKey()
	sign := crypto.NewSigner(pub, priv)
	fmt.Println("Signer is: ", sign)
	// (FfiStr submitter_did,
	// 	FfiStr dest,
	// 	FfiStr verkey,
	// 	FfiStr alias,
	// 	FfiStr role,
	// 	FfiStr diddoc_content,
	// 	int32_t version,
	// 	RequestHandle *handle_p);
	var cusreq C.int64_t
	var none64 C.int32_t = 2
	diddoc := `
	{
		"@context": [
		  "https://www.w3.org/ns/did/v1",
		  "https://w3id.org/security/suites/ed25519-2020/v1"
		],
		"authentication": [{
		  "id": "2sz51n1iryuv5shFN3PuNfDR1FEsPXt9yWvfHc4hbabA#keys-1",
		  "type": "Ed25519VerificationKey2020",
		  "controller": "2sz51n1iryuv5shFN3PuNfDR1FEsPXt9yWvfHc4hbabA",
		  "publicKeyMultibase": "3r27cGC7WpsttqRmDx9DLGEsHodF8uE72b7rAfw7xzfM"
		}]
	  }
	  `
	resukt := C.indy_vdr_build_nym_request(C.CString("J5rfgYns3cdRvg1rUVJbYf5ovr9pgUVv8eF38zRd3GJF"), C.CString(authorDID), C.CString(authorPubKey), C.CString(""), C.CString(""), C.CString(diddoc), none64, &cusreq)

	if resukt != 0 {
		var errMsg *C.char
		C.indy_vdr_get_current_error(&errMsg)
		defer C.free(unsafe.Pointer(errMsg))
		fmt.Printf("invalid custom request: (Indy error code: [%s])", C.GoString(errMsg))
		return
	}
	fmt.Println("\n\nNYM request built by the ledger is: ", resukt)
	defer C.indy_vdr_request_free(cusreq)
	// nym := NewNym(authorDID, authorPubKey, "", "J5rfgYns3cdRvg1rUVJbYf5ovr9pgUVv8eF38zRd3GJF", "")
	cursbyte, err := json.MarshalIndent(cusreq, " ", "")
	if err != nil {
		panic(err)
	}
	byt := new(bytes.Buffer)
	sig, _ := sign.Sign(cursbyte)
	byt.Write(sig)
	// intVal := int64(binary.BigEndian.Uint64(request))
	// length := len(request)
	// lenbytes, _ := json.Marshal(length)
	// intValLen := (*C.int64_t)(unsafe.Pointer(&lenbytes))
	data8 := (*C.uint8_t)(unsafe.Pointer(&cursbyte))
	buf := C.ByteBuffer{len: C.int64_t(len(sig)), data: data8}

	// Now we can pass intVal to the C function, but we need to convert it to _Ctype_longlong
	//C.print_long_long(C.longlong(intVal))
	reply := C.indy_vdr_request_set_signature(cusreq, buf)
	if reply != 0 {
		var errMsg *C.char
		C.indy_vdr_get_current_error(&errMsg)
		defer C.free(unsafe.Pointer(errMsg))
		fmt.Printf("invalid custom writerequest: (Indy error code: [%s])", C.GoString(errMsg))
		return
	}

	fmt.Println("\n\nAfter setting signature: ", cusreq)
	//sign with endorser key

	priv, _ = base58.Decode("65M4JuzSC4eZ1M3c7QGzME7smQXoHK7QBDL7p2UCxKJQS4QWAVT3ec5dVmb2Dk6499FFEpGBmQX9VKAXiCz3JRLa")
	pub, _ = base58.Decode("DRUe4EUorCzB2oyRH3WMkDc8rFgx2xmigTH5eapodmCi")

	signEndo := crypto.NewSigner(pub, priv)
	fmt.Println("Signer is: ", sign)
	cursbyte, err = json.MarshalIndent(cusreq, " ", "")
	if err != nil {
		panic(err)
	}
	byt = new(bytes.Buffer)
	sig, _ = signEndo.Sign(cursbyte)
	byt.Write(sig)
	// intVal := int64(binary.BigEndian.Uint64(request))
	// length := len(request)
	// lenbytes, _ := json.Marshal(length)
	// intValLen := (*C.int64_t)(unsafe.Pointer(&lenbytes))
	data8 = (*C.uint8_t)(unsafe.Pointer(&cursbyte))
	buf = C.ByteBuffer{len: C.int64_t(len(sig)), data: data8}

	resp := C.indy_vdr_request_set_multi_signature(cusreq, C.CString("J5rfgYns3cdRvg1rUVJbYf5ovr9pgUVv8eF38zRd3GJF"), buf)
	if resp != 0 {
		var errMsg *C.char
		C.indy_vdr_get_current_error(&errMsg)
		defer C.free(unsafe.Pointer(errMsg))
		fmt.Printf("invalid custom writerequest: (Indy error code: [%s])", C.GoString(errMsg))
		return
	}
	fmt.Println("\n\nAfter setting multi signature: ", cusreq)
	rep, err := r.submitReadRequest(resp)
	if err != nil {
		panic(err)
	}

	// rply, err := parseWriteReply(rep.)
	// if err != nil {
	// 	return nil, err
	// }
	fmt.Println("reply is -----> ", rep)
	//C.indy_vdr_request_set_multi_signature()
}

func (r *Client) AddNewContext(id string) (*ReadReply, error) {
	fmt.Println("------------- Adding Context -------------")
	//var schreq *C.int64_t
	//	submitterDID := ""
	content := `
{
  "@context": {
    "@protected": true,
    "@version": 1.1,
    "IndividualHandle": {
     "@context": {
      "@protected": true,
      "@version": 1.1,
      "schema": "http://schema.org/",
      "handleIdentifier": "schema:handleIdentifier"
    },
    "@id": "https://blockchaingateway.qikfox.com/getContext/IndividualHandleUserProfileCredentialContext.jsonld#IndividualHandle"
    },
    "PII": {
      "@context": {
        "@protected": true,
        "@version": 1.1,
        "address": "schema:address",
        "cid": "schema:cid",
        "defaultValue": "schema:defaultValue",
        "description": "schema:description",
        "email": "schema:email",
        "facebook": "schema:facebook",
        "familyName": "schema:familyName",
        "givenName": "schema:givenName",
        "gmail": "schema:gmail",
        "handle": "schema:handle",
        "id": "@id",
        "instagram": "schema:instagram",
        "ipAddress": "schema:ipAddress",
        "linkedin": "schema:linkedin",
        "private": "schema:private",
        "public": "schema:public",
        "schema": "http://schema.org/",
        "stacks": "schema:stacks",
        "telephone": "schema:telephone",
        "twitter": "schema:twitter",
        "type": "@type"
      },
      "@id": "https://blockchaingateway.qikfox.com/getContext/IndividualHandleUserProfileCredentialContext.jsonld#PII"
    },
    "SelfSignedCredential": {
      "@context": {
        "@version": 1.1,
        "id": "@id",
        "type": "@type"
      },
      "@id": "https://blockchaingateway.qikfox.com/getContext/IndividualHandleUserProfileCredentialContext.jsonld#SelfSignedCredential"
    },
    "UserInfoCredential": {
      "@context": {
        "@protected": true,
        "@version": 1.1,
        "id": "@id",
        "identifier": "http://schema.org/identifier",
        "name": "http://schema.org/name",
        "type": "@type"
      },
      "@id": "https://blockchaingateway.qikfox.com/getContext/IndividualHandleUserProfileCredentialContext.jsonld#UserInfoCredential"
    },
    "identifier": "http://schema.org/identifier",
    "name": "http://schema.org/name"
  }
}
`
	// var none *C.char
	// var none32 C.int32_t = -1 // seq_no
	// var none64 C.int64_t = -1 // timestamp
	//cdid := C.CString(id)
	hasher := sha256.New()
	hasher.Write([]byte("arunsteward000000000000000000000"))
	seedBytes := hasher.Sum(nil)
	priv := ed25519.NewKeyFromSeed(seedBytes)
	pub := priv.Public().(ed25519.PublicKey)
	// pr, _ := base58.Decode("2x11kq6QjK1GBuSv84pGj2vwyRVR5FDyYeKzw6mMJXecGUfqB6ypos3Q4utFfvde7uKtNzDxirXXbrvhLxCnMvYM")
	// prForSign := ed25519.PrivateKey(pr)
	// //privkey := ed25519.NewKeyFromSeed(seed)
	// pubForSign := prForSign.Public().(ed25519.PublicKey)
	encPub := base64.RawURLEncoding.EncodeToString(pub)
	fmt.Println(" ---- > Pub key", encPub)
	sign := crypto.NewSigner(pub, priv)

	// fmt.Println("---- Private Key ---- ", base58.Encode(prForSign))
	// fmt.Println("---- Public Key ---- ", base58.Encode(pubkey))
	// //	pub, priv, _ := ed25519.GenerateKey()
	// sign := crypto.NewSigner(pubkey, prForSign)
	fmt.Println("Signer is: ", sign)
	// privkey := ed25519.NewKeyFromSeed(seed)
	// fmt.Println("Private key is: ", privkey)
	// pub, priv, err := ed25519.GenerateKey(seed)
	// dec, err := base58.Decode("6wcFJttUESFzsXPH9cnjqaWSTkYyrz9sV5GvvcXrZwCa")
	// if err != nil {
	// 	fmt.Println("Error is: ", err)
	// 	return nil, nil
	// }
	// pubKey := ed25519.PublicKey(dec)
	reply, err := r.CreateJsonldContext("IndividualHandleUserProfileCredentialContext", "IndividualHandleUserProfileCredentialContext", "1.0", content, sign)
	if err != nil {
		fmt.Println("Error in adding rich schema: ", err)
		return nil, err
	}
	fmt.Println("Response is: ", reply)
	//fmt.Println("Rich schema added successfully: ", txnId)
	fmt.Println("--------------------------------")
	return nil, nil
}

func (r *Client) AddRichSchema(id string) (*ReadReply, error) {
	fmt.Println("------------- Adding Rich Schema -------------")
	//var schreq *C.int64_t
	//	submitterDID := ""
	cont := `
{
  "$schema": "http://json-schema.org/draft-07/schema#",
  "@id":"did:fox:DttBXwWJjKw5yrYHCGtSqP#IndividualHandleUserProfileCredentialSchema",
  "@type":"rdfs:Class",
  "type": "object",
  "properties": {
    "@context": {
      "type": "array",
      "items": {
        "type": "string",
        "format": "uri"
      }
    },
    "credentialSchema": {
      "type": "object",
      "properties": {
        "id": {
          "type": "string",
          "format": "uri"
        },
        "type": {
          "type": "string"
        }
      },
      "required": ["id", "type"]
    },
    "credentialSubject": {
      "type": "object",
      "properties": {
        "familyName": {
          "type": "string"
        },
        "givenName": {
          "type": "string"
        },
        "email": {
          "type": "string"
        },
        "telephone": {
          "type": "string"
        },
        "address": {
          "type": "string"
        },
        "description": {
          "type": "string"
        },
        "ipAddress": {
          "type": "string"
        },
        "stacks": {
          "type": "string"
        },
        "linkedin": {
          "type": "string"
        },
        "twitter": {
          "type": "string"
        },
        "facebook": {
          "type": "string"
        },
        "instagram": {
          "type": "string"
        },
        "gmail": {
          "type": "string"
        },
        "cid": {
          "type": "string"
        },
        "handle": {
          "type": "string"
        },
        "defaultValue": {
          "type": "string"
        },
        "id": {
          "type": "string",
          "format": "uri"
        },
        "public": {
          "type": "string"
        },
        "private": {
          "type": "string"
        },
        "handleIdentifier": {
          "type": "string"
        },
        "type": {
          "type": "array",
          "items": {
            "type": "string"
          }
        }
      },
      "required": [
        "id", "type"
      ]
    },
    "id": {
      "type": "string",
      "format": "uri"
    },
    "type": {
      "type": "array",
      "items": {
        "type": "string"
      }
    },
    "issuer": {
      "type": "string",
      "format": "uri"
    },
    "identifier": {
      "type": "string",
      "format": "uri"
    },
    "name": {
      "type": "string"
    },
    "issuanceDate": {
      "type": "string",
      "format": "date-time"
    }
  },
  "required": [
    "@context", "credentialSchema", "credentialSubject", "id", "type", "issuer", 
    "identifier", "name", "issuanceDate"
  ]
}
	`
	// var none *C.char
	// var none32 C.int32_t = -1 // seq_no
	// var none64 C.int64_t = -1 // timestamp
	//cdid := C.CString(id)
	// seed, err := identifiers.ConvertSeed("arunsteward000000000000000000000")
	// if err != nil {
	// 	log.Fatalln(err)
	// }
	// privkey := ed25519.NewKeyFromSeed(seed)
	// fmt.Println("Private key is: ", privkey)
	// pub, priv, err := ed25519.GenerateKey(seed)
	// dec, err := base58.Decode("6wcFJttUESFzsXPH9cnjqaWSTkYyrz9sV5GvvcXrZwCa")
	// if err != nil {
	// 	fmt.Println("Error is: ", err)
	// 	return nil, nil
	// }
	// pubKey := ed25519.PublicKey(dec)
	hasher := sha256.New()
	hasher.Write([]byte("arunsteward000000000000000000000"))
	seedBytes := hasher.Sum(nil)
	priv := ed25519.NewKeyFromSeed(seedBytes)
	pub := priv.Public().(ed25519.PublicKey)
	// pr, _ := base58.Decode("2x11kq6QjK1GBuSv84pGj2vwyRVR5FDyYeKzw6mMJXecGUfqB6ypos3Q4utFfvde7uKtNzDxirXXbrvhLxCnMvYM")
	// prForSign := ed25519.PrivateKey(pr)
	// //privkey := ed25519.NewKeyFromSeed(seed)
	// pubForSign := prForSign.Public().(ed25519.PublicKey)
	encPub := base64.RawURLEncoding.EncodeToString(pub)
	fmt.Println(" ---- > Pub key", encPub)
	sign := crypto.NewSigner(pub, priv)

	// fmt.Println("---- Private Key ---- ", base58.Encode(prForSign))
	// fmt.Println("---- Public Key ---- ", base58.Encode(pubkey))
	// //	pub, priv, _ := ed25519.GenerateKey()
	// sign := crypto.NewSigner(pubkey, prForSign)
	fmt.Println("Signer is: ", sign)
	reply, err := r.CreateRichSchema("did:fox:DttBXwWJjKw5yrYHCGtSqP#IndividualHandleUserProfileCredentialSchema", "IndividualHandleUserProfileCredentialSchema", "1.0", cont, sign)
	if err != nil {
		fmt.Println("Error in adding rich schema: ", err)
		return nil, err
	}
	fmt.Println("Response is: ", reply)
	//fmt.Println("Rich schema added successfully: ", txnId)
	fmt.Println("--------------------------------")
	return nil, nil
	// uuid := "efuihyy83yrfhuyef"
	// //var idstr, uuidCstr, contentCstr, nameCstr, verCstr, prtCstr, typeCstr *_Ctype_char
	// idstr := C.CString(id)
	// defer C.free(unsafe.Pointer(idstr))
	// uuidCstr := C.CString(uuid)
	// contentCstr := C.CString(content)
	// nameCstr := C.CString("TestSchema")
	// verCstr := C.CString("1.0")
	// prtCstr := C.CString("2.0")
	// typeCstr := C.CString("sch")
	// defer C.free(unsafe.Pointer(uuidCstr))
	// defer C.free(unsafe.Pointer(contentCstr))
	// defer C.free(unsafe.Pointer(nameCstr))
	// defer C.free(unsafe.Pointer(verCstr))
	// defer C.free(unsafe.Pointer(prtCstr))
	// defer C.free(unsafe.Pointer(typeCstr))
	// //result := C.indy_vdr_build_rich_schema_request(idstr, C.CString("efuihyy83yrfhuyef"), C.CString(content), C.CString("TestSchema"), C.CString("1.0"), C.CString("sch"), C.CString("2.0"), schreq)
	// result := C.indy_vdr_build_rich_schema_request(idstr, uuidCstr, contentCstr, nameCstr, verCstr, typeCstr, verCstr, schreq)

	// //result = C.indy_vdr_build_get_nym_request(none, cdid, none32, none64, &nymreq)
	// //C.free(unsafe.Pointer(id))
	// if result != 0 {
	// 	fmt.Println("Error is: ", result)
	// 	return nil, fmt.Errorf("invalid get rich schema request: (Indy error code: [%v])", result)
	// }
	// defer C.indy_vdr_request_free(*schreq)
	// resp, err := r.submitReadRequest(*schreq)
	// if err != nil {
	// 	fmt.Println("Error ----> ", err)
	// 	return nil, nil
	// }
	// fmt.Println("schema added is: ", resp)
	// return resp, nil
}
